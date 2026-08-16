//! Tmux Terminal Multiplexer for Pico OS
//! Supports 1 to 4 split-screen panes (Vertical, Horizontal, 2x2 Grid),
//! active pane switching, independent command buffers, and a live status bar.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::shell::commands::{execute_command, CommandContext};
use crate::task;

const MAX_PANES: usize = 4;
const MAX_PANE_LINES: usize = 12;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SplitMode {
    Single,     // 1 Pane
    Horizontal, // 2 Panes stacked (Top / Bottom)
    Vertical,   // 2 Panes side-by-side (Left / Right)
    Triple,     // 2 Top + 1 Bottom
    Grid4,      // 2x2 Grid
}

pub struct Pane {
    pub id: usize,
    pub name: String,
    pub input_buffer: String,
    pub history: Vec<String>,
    pub screen_lines: Vec<String>,
}

impl Pane {
    pub fn new(id: usize, name: &str) -> Self {
        let mut screen_lines = Vec::new();
        screen_lines.push(format!("\x1b[1;36m[Pane {}: {} active]\x1b[0m", id, name));
        screen_lines.push(String::from("Type 'split-h', 'split-v', or Ctrl+B % / \" to split."));

        Pane {
            id,
            name: String::from(name),
            input_buffer: String::new(),
            history: Vec::new(),
            screen_lines,
        }
    }
}

pub struct TmuxManager {
    pub panes: Vec<Pane>,
    pub active_idx: usize,
    pub split_mode: SplitMode,
    pub prefix_active: bool,
    pub should_exit: bool,
    pub status_message: Option<String>,
}

impl TmuxManager {
    pub fn new() -> Self {
        let mut panes = Vec::new();
        panes.push(Pane::new(1, "sh"));

        TmuxManager {
            panes,
            active_idx: 0,
            split_mode: SplitMode::Single,
            prefix_active: false,
            should_exit: false,
            status_message: Some(String::from("tmux 4-Pane Multiplexer ready")),
        }
    }

    /// Add a new pane and automatically adjust split layout
    pub fn split_pane(&mut self, is_vertical: bool) -> bool {
        if self.panes.len() >= MAX_PANES {
            self.status_message = Some(String::from("Maximum 4 panes reached"));
            return false;
        }

        let new_id = self.panes.len() + 1;
        let name = format!("sh{}", new_id);
        self.panes.push(Pane::new(new_id, &name));
        self.active_idx = self.panes.len() - 1;

        self.split_mode = match self.panes.len() {
            1 => SplitMode::Single,
            2 => if is_vertical { SplitMode::Vertical } else { SplitMode::Horizontal },
            3 => SplitMode::Triple,
            _ => SplitMode::Grid4,
        };

        self.status_message = Some(format!("Split: Pane {} created", new_id));
        true
    }

    /// Close active pane
    pub fn close_active_pane(&mut self) {
        if self.panes.len() > 1 {
            self.panes.remove(self.active_idx);
            if self.active_idx >= self.panes.len() {
                self.active_idx = self.panes.len() - 1;
            }
            // Re-assign IDs
            for (i, p) in self.panes.iter_mut().enumerate() {
                p.id = i + 1;
            }
            self.split_mode = match self.panes.len() {
                1 => SplitMode::Single,
                2 => SplitMode::Vertical,
                3 => SplitMode::Triple,
                _ => SplitMode::Grid4,
            };
            self.status_message = Some(String::from("Pane closed"));
        } else {
            self.should_exit = true;
        }
    }

    /// Render full terminal screen including split pane borders & content
    pub fn render<F: FnMut(&str)>(&mut self, mut write_out: F) {
        // Clear screen and hide cursor while drawing
        write_out("\x1b[?25l\x1b[2J\x1b[H");

        match self.panes.len() {
            1 => self.render_single(&mut write_out),
            2 => {
                if self.split_mode == SplitMode::Horizontal {
                    self.render_2_horizontal(&mut write_out);
                } else {
                    self.render_2_vertical(&mut write_out);
                }
            }
            3 => self.render_3_panes(&mut write_out),
            _ => self.render_4_grid(&mut write_out),
        }

        self.render_status_bar(&mut write_out);
        write_out("\x1b[?25h"); // Restore cursor
    }

    fn render_single<F: FnMut(&str)>(&self, mut write_out: F) {
        if let Some(p) = self.panes.first() {
            write_out("\x1b[1;32m┌── [Pane 1: sh *] ────────────────────────────────────────────────────────┐\x1b[0m\r\n");
            for line in &p.screen_lines {
                write_out("│ ");
                write_out(line);
                write_out("\x1b[K\r\n");
            }
            // Empty space padding
            for _ in p.screen_lines.len()..16 {
                write_out("│\x1b[K\r\n");
            }
            write_out("├──\x1b[K\r\n");
            let prompt = format!("│ \x1b[1;32mroot@pico\x1b[0m:\x1b[1;34m[pane1]\x1b[0m# {}\x1b[K\r\n", p.input_buffer);
            write_out(&prompt);
            write_out("\x1b[1;32m└──────────────────────────────────────────────────────────────────────────┘\x1b[0m\r\n");
        }
    }

    fn render_2_vertical<F: FnMut(&str)>(&self, mut write_out: F) {
        let p1 = &self.panes[0];
        let p2 = &self.panes[1];
        let a1 = if self.active_idx == 0 { "\x1b[1;32m* Active\x1b[0m" } else { "        " };
        let a2 = if self.active_idx == 1 { "\x1b[1;32m* Active\x1b[0m" } else { "        " };

        let header = format!(
            "\x1b[1;36m┌── [Pane 1: {}] ───────┐┌── [Pane 2: {}] ───────┐\x1b[0m\r\n",
            a1, a2
        );
        write_out(&header);

        for row in 0..14 {
            let l1 = p1.screen_lines.get(row).map(|s| s.as_str()).unwrap_or("");
            let l2 = p2.screen_lines.get(row).map(|s| s.as_str()).unwrap_or("");
            let line = format!("│ {:<34} ││ {:<34} │\r\n", Self::truncate_clean(l1, 34), Self::truncate_clean(l2, 34));
            write_out(&line);
        }

        let in1 = format!("> {}", p1.input_buffer);
        let in2 = format!("> {}", p2.input_buffer);
        let in_row = format!("│ {:<34} ││ {:<34} │\r\n", Self::truncate_clean(&in1, 34), Self::truncate_clean(&in2, 34));
        write_out(&in_row);
        write_out("\x1b[1;36m└────────────────────────────────────┘└─── ────────────────────────────────┘\x1b[0m\r\n");
    }

    fn render_2_horizontal<F: FnMut(&str)>(&self, mut write_out: F) {
        for (i, p) in self.panes.iter().enumerate() {
            let active = if i == self.active_idx { "\x1b[1;32m* Active\x1b[0m" } else { "        " };
            let head = format!("\x1b[1;36m┌── [Pane {}: {}] ────────────────────────────────────────────────────────┐\x1b[0m\r\n", p.id, active);
            write_out(&head);
            for row in 0..6 {
                let l = p.screen_lines.get(row).map(|s| s.as_str()).unwrap_or("");
                let line = format!("│ {:<72} │\r\n", Self::truncate_clean(l, 72));
                write_out(&line);
            }
            let in_str = format!("> {}", p.input_buffer);
            let in_row = format!("│ {:<72} │\r\n", Self::truncate_clean(&in_str, 72));
            write_out(&in_row);
            write_out("\x1b[1;36m└──────────────────────────────────────────────────────────────────────────┘\x1b[0m\r\n");
        }
    }

    fn render_3_panes<F: FnMut(&str)>(&self, mut write_out: F) {
        // Top 2 vertical + Bottom 1 horizontal
        let p1 = &self.panes[0];
        let p2 = &self.panes[1];
        let p3 = &self.panes[2];

        let a1 = if self.active_idx == 0 { "\x1b[1;32m* Active\x1b[0m" } else { "        " };
        let a2 = if self.active_idx == 1 { "\x1b[1;32m* Active\x1b[0m" } else { "        " };
        let a3 = if self.active_idx == 2 { "\x1b[1;32m* Active\x1b[0m" } else { "        " };

        let head = format!("\x1b[1;36m┌── [Pane 1: {}] ───────┐┌── [Pane 2: {}] ───────┐\x1b[0m\r\n", a1, a2);
        write_out(&head);

        for row in 0..6 {
            let l1 = p1.screen_lines.get(row).map(|s| s.as_str()).unwrap_or("");
            let l2 = p2.screen_lines.get(row).map(|s| s.as_str()).unwrap_or("");
            let line = format!("│ {:<34} ││ {:<34} │\r\n", Self::truncate_clean(l1, 34), Self::truncate_clean(l2, 34));
            write_out(&line);
        }
        let in1 = format!("> {}", p1.input_buffer);
        let in2 = format!("> {}", p2.input_buffer);
        write_out(&format!("│ {:<34} ││ {:<34} │\r\n", Self::truncate_clean(&in1, 34), Self::truncate_clean(&in2, 34)));
        write_out("\x1b[1;36m└─── ────────────────────────────────┘└─── ────────────────────────────────┘\x1b[0m\r\n");

        // Bottom Pane 3
        let head3 = format!("\x1b[1;36m┌── [Pane 3: {}] ────────────────────────────────────────────────────────┐\x1b[0m\r\n", a3);
        write_out(&head3);
        for row in 0..4 {
            let l3 = p3.screen_lines.get(row).map(|s| s.as_str()).unwrap_or("");
            write_out(&format!("│ {:<72} │\r\n", Self::truncate_clean(l3, 72)));
        }
        let in3 = format!("> {}", p3.input_buffer);
        write_out(&format!("│ {:<72} │\r\n", Self::truncate_clean(&in3, 72)));
        write_out("\x1b[1;36m└──────────────────────────────────────────────────────────────────────────┘\x1b[0m\r\n");
    }

    fn render_4_grid<F: FnMut(&str)>(&self, mut write_out: F) {
        let p1 = &self.panes[0];
        let p2 = &self.panes[1];
        let p3 = &self.panes[2];
        let p4 = &self.panes[3];

        let a1 = if self.active_idx == 0 { "\x1b[1;32m* P1\x1b[0m" } else { "  P1" };
        let a2 = if self.active_idx == 1 { "\x1b[1;32m* P2\x1b[0m" } else { "  P2" };
        let a3 = if self.active_idx == 2 { "\x1b[1;32m* P3\x1b[0m" } else { "  P3" };
        let a4 = if self.active_idx == 3 { "\x1b[1;32m* P4\x1b[0m" } else { "  P4" };

        // Top Half
        write_out(&format!("\x1b[1;36m┌── [{}] ───────────────────┐┌── [{}] ───────────────────┐\x1b[0m\r\n", a1, a2));
        for row in 0..5 {
            let l1 = p1.screen_lines.get(row).map(|s| s.as_str()).unwrap_or("");
            let l2 = p2.screen_lines.get(row).map(|s| s.as_str()).unwrap_or("");
            write_out(&format!("│ {:<34} ││ {:<34} │\r\n", Self::truncate_clean(l1, 34), Self::truncate_clean(l2, 34)));
        }
        let in1 = format!("> {}", p1.input_buffer);
        let in2 = format!("> {}", p2.input_buffer);
        write_out(&format!("│ {:<34} ││ {:<34} │\r\n", Self::truncate_clean(&in1, 34), Self::truncate_clean(&in2, 34)));
        write_out("\x1b[1;36m└────────────────────────────────────┘└─── ────────────────────────────────┘\x1b[0m\r\n");

        // Bottom Half
        write_out(&format!("\x1b[1;36m┌── [{}] ───────────────────┐┌── [{}] ───────────────────┐\x1b[0m\r\n", a3, a4));
        for row in 0..5 {
            let l3 = p3.screen_lines.get(row).map(|s| s.as_str()).unwrap_or("");
            let l4 = p4.screen_lines.get(row).map(|s| s.as_str()).unwrap_or("");
            write_out(&format!("│ {:<34} ││ {:<34} │\r\n", Self::truncate_clean(l3, 34), Self::truncate_clean(l4, 34)));
        }
        let in3 = format!("> {}", p3.input_buffer);
        let in4 = format!("> {}", p4.input_buffer);
        write_out(&format!("│ {:<34} ││ {:<34} │\r\n", Self::truncate_clean(&in3, 34), Self::truncate_clean(&in4, 34)));
        write_out("\x1b[1;36m└────────────────────────────────────┘└─── ────────────────────────────────┘\x1b[0m\r\n");
    }

    fn truncate_clean(s: &str, max_len: usize) -> String {
        // Strip ANSI codes for length counting if needed, or simple char take
        let mut clean = String::new();
        let mut count = 0;
        let mut in_escape = false;

        for c in s.chars() {
            if c == '\x1b' {
                in_escape = true;
                clean.push(c);
            } else if in_escape {
                clean.push(c);
                if c == 'm' {
                    in_escape = false;
                }
            } else {
                if count < max_len {
                    clean.push(c);
                    count += 1;
                } else {
                    break;
                }
            }
        }
        clean
    }

    /// Render green bottom status bar
    pub fn render_status_bar<F: FnMut(&str)>(&self, mut write_out: F) {
        let ticks = task::get_uptime_ticks();
        let secs = ticks / 1000;
        let mins = (secs / 60) % 60;
        let s = secs % 60;

        write_out("\x1b[s\x1b[24;1H\x1b[42;30m"); // Green bar at bottom row
        write_out("[pico-tmux] ");

        let pane_info = format!("Panes: {}/4 (Active: Pane {}) | ", self.panes.len(), self.active_idx + 1);
        write_out(&pane_info);

        if self.prefix_active {
            write_out("\x1b[43;30m<Ctrl+B PREFIX>\x1b[42;30m ");
        } else if let Some(ref msg) = self.status_message {
            let msg_fmt = format!("({}) ", msg);
            write_out(&msg_fmt);
        }

        let right_info = format!(" \"RP2040 SMP\" {:02}:{:02}\x1b[K\x1b[0m\x1b[u", mins, s);
        write_out(&right_info);
    }

    pub fn handle_key<F: FnMut(&str)>(&mut self, byte: u8, mut write_out: F) {
        // Check for Prefix key: Ctrl+B (0x02) or Ctrl+A (0x01)
        if byte == 0x02 || byte == 0x01 {
            self.prefix_active = true;
            self.status_message = None;
            self.render_status_bar(&mut write_out);
            return;
        }

        // If prefix is currently active, execute multiplexer command
        if self.prefix_active {
            self.prefix_active = false;
            match byte {
                // '%' or 'v' or 'V' or 'r': Split Vertical (Right)
                b'%' | b'v' | b'V' | b'r' | b'R' => {
                    self.split_pane(true);
                    self.render(&mut write_out);
                    return;
                }
                // '"' or 'h' or 'H' or 'd': Split Horizontal (Down)
                b'"' | b'h' | b'H' | b'D' => {
                    self.split_pane(false);
                    self.render(&mut write_out);
                    return;
                }
                // 'o' or '\t': Next pane
                b'o' | b'O' | b'\t' => {
                    if !self.panes.is_empty() {
                        self.active_idx = (self.active_idx + 1) % self.panes.len();
                        self.status_message = Some(format!("Switched to Pane {}", self.active_idx + 1));
                        self.render(&mut write_out);
                    }
                    return;
                }
                // '1'..='4': Select pane directly
                b'1'..=b'4' => {
                    let idx = (byte - b'1') as usize;
                    if idx < self.panes.len() {
                        self.active_idx = idx;
                        self.status_message = Some(format!("Switched to Pane {}", idx + 1));
                        self.render(&mut write_out);
                    }
                    return;
                }
                // 'x' or 'k': Close active pane
                b'x' | b'X' | b'k' | b'K' => {
                    self.close_active_pane();
                    self.render(&mut write_out);
                    return;
                }
                // 'd': Detach
                b'd' => {
                    self.should_exit = true;
                    return;
                }
                // '?': Help overlay
                b'?' => {
                    if let Some(p) = self.panes.get_mut(self.active_idx) {
                        p.screen_lines.push(String::from("\x1b[1;36m=== Tmux Pane Shortcuts ===\x1b[0m"));
                        p.screen_lines.push(String::from("  Ctrl+B % / v : Split Vertical (Right)"));
                        p.screen_lines.push(String::from("  Ctrl+B \" / h : Split Horizontal (Down)"));
                        p.screen_lines.push(String::from("  Ctrl+B o / Tab: Cycle active pane"));
                        p.screen_lines.push(String::from("  Ctrl+B 1..4  : Select Pane 1 to 4"));
                        p.screen_lines.push(String::from("  Ctrl+B x     : Close current pane"));
                        p.screen_lines.push(String::from("  Ctrl+B d     : Detach back to OS shell"));
                    }
                    self.render(&mut write_out);
                    return;
                }
                _ => {
                    self.status_message = Some(String::from("Unknown command. Press '?' for help."));
                    self.render_status_bar(&mut write_out);
                    return;
                }
            }
        }

        // Standard terminal typing inside active pane
        let active_idx = self.active_idx;
        if active_idx >= self.panes.len() {
            return;
        }

        match byte {
            // Enter key
            b'\r' | b'\n' => {
                let cmd_line = self.panes[active_idx].input_buffer.trim().to_string();
                self.panes[active_idx].input_buffer.clear();

                if cmd_line == "exit" || cmd_line == "quit" {
                    self.close_active_pane();
                    self.render(&mut write_out);
                    return;
                }

                if cmd_line == "clear" {
                    self.panes[active_idx].screen_lines.clear();
                    self.render(&mut write_out);
                    return;
                }

                // Tmux shell commands
                if cmd_line == "split-v" || cmd_line == "split-right" || cmd_line == "split right" || cmd_line == "split" {
                    self.split_pane(true);
                    self.render(&mut write_out);
                    return;
                }

                if cmd_line == "split-h" || cmd_line == "split-down" || cmd_line == "split down" {
                    self.split_pane(false);
                    self.render(&mut write_out);
                    return;
                }

                if cmd_line.starts_with("focus ") || cmd_line.starts_with("pane ") {
                    let parts: Vec<&str> = cmd_line.split_whitespace().collect();
                    if let Some(num_str) = parts.get(1) {
                        if let Ok(num) = num_str.parse::<usize>() {
                            if num >= 1 && num <= self.panes.len() {
                                self.active_idx = num - 1;
                                self.status_message = Some(format!("Focused on Pane {}", num));
                                self.render(&mut write_out);
                                return;
                            }
                        }
                    }
                }

                let prompt_line = format!("$ {}", cmd_line);
                self.panes[active_idx].screen_lines.push(prompt_line);

                if !cmd_line.is_empty() {
                    self.panes[active_idx].history.push(cmd_line.clone());

                    // Execute command and capture output into pane screen
                    let mut cmd_output: Vec<String> = Vec::new();
                    {
                        let mut ctx = CommandContext {
                            output: &mut |s| {
                                for line in s.split('\n') {
                                    let trimmed = line.trim_end_matches('\r');
                                    if !trimmed.is_empty() {
                                        cmd_output.push(String::from(trimmed));
                                    }
                                }
                            },
                        };
                        execute_command(&cmd_line, &mut ctx);
                    }

                    self.panes[active_idx].screen_lines.extend(cmd_output);
                }

                // Keep recent lines
                while self.panes[active_idx].screen_lines.len() > MAX_PANE_LINES {
                    self.panes[active_idx].screen_lines.remove(0);
                }

                self.render(&mut write_out);
            }
            // Backspace
            0x08 | 0x7F => {
                if !self.panes[active_idx].input_buffer.is_empty() {
                    self.panes[active_idx].input_buffer.pop();
                    self.render(&mut write_out);
                }
            }
            // Ctrl+C
            0x03 => {
                self.panes[active_idx].input_buffer.clear();
                self.panes[active_idx].screen_lines.push(String::from("^C"));
                self.render(&mut write_out);
            }
            // Ctrl+L
            0x0C => {
                self.panes[active_idx].screen_lines.clear();
                self.render(&mut write_out);
            }
            // Printable ASCII
            32..=126 => {
                let ch = byte as char;
                self.panes[active_idx].input_buffer.push(ch);
                self.render(&mut write_out);
            }
            _ => {}
        }
    }
}
