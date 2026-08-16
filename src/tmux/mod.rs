//! Tmux Terminal Multiplexer for Pico OS
//! Provides multi-window virtual terminal sessions, status bar, and prefix key navigation.

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::shell::commands::{execute_command, CommandContext};
use crate::task;

const MAX_WINDOWS: usize = 4;
const MAX_SCREEN_LINES: usize = 20;

pub struct TmuxWindow {
    pub name: String,
    pub input_buffer: String,
    pub history: Vec<String>,
    pub screen_lines: Vec<String>,
}

impl TmuxWindow {
    pub fn new(name: &str) -> Self {
        let mut screen_lines = Vec::new();
        screen_lines.push(format!("\x1b[1;36m[tmux: window '{}' initialized]\x1b[0m", name));
        screen_lines.push(String::from("Type commands or use \x1b[1;33mCtrl+B c\x1b[0m (new window), \x1b[1;33mCtrl+B n/p\x1b[0m (switch), \x1b[1;33mCtrl+B d\x1b[0m (detach)."));
        screen_lines.push(String::new());

        TmuxWindow {
            name: String::from(name),
            input_buffer: String::new(),
            history: Vec::new(),
            screen_lines,
        }
    }
}

pub struct TmuxManager {
    pub windows: Vec<TmuxWindow>,
    pub active_idx: usize,
    pub prefix_active: bool,
    pub should_exit: bool,
    pub status_message: Option<String>,
}

impl TmuxManager {
    pub fn new() -> Self {
        let mut windows = Vec::new();
        windows.push(TmuxWindow::new("sh"));

        TmuxManager {
            windows,
            active_idx: 0,
            prefix_active: false,
            should_exit: false,
            status_message: Some(String::from("tmux ready - Prefix is [Ctrl+B]")),
        }
    }

    /// Full re-render of current window screen and bottom status bar
    pub fn render<F: FnMut(&str)>(&mut self, mut write_out: F) {
        // Clear screen and move cursor to top-left
        write_out("\x1b[2J\x1b[H");

        // Render current window content
        if let Some(win) = self.windows.get(self.active_idx) {
            for line in &win.screen_lines {
                write_out(line);
                write_out("\r\n");
            }
            // Prompt and current input buffer
            write_out("\x1b[1;32mroot@pico\x1b[0m:\x1b[1;34m[tmux]\x1b[0m# ");
            write_out(&win.input_buffer);
        }

        self.render_status_bar(&mut write_out);
    }

    /// Render green bottom status bar
    pub fn render_status_bar<F: FnMut(&str)>(&self, mut write_out: F) {
        let ticks = task::get_uptime_ticks();
        let secs = ticks / 1000;
        let mins = (secs / 60) % 60;
        let s = secs % 60;

        // Save cursor, jump to row 24 (bottom), render green bar, restore cursor
        write_out("\x1b[s\x1b[24;1H\x1b[42;30m"); // Green background, black text

        write_out("[pico] ");

        // List window tabs
        for (i, win) in self.windows.iter().enumerate() {
            if i == self.active_idx {
                let tab = format!("{}:{}* ", i, win.name);
                write_out(&tab);
            } else {
                let tab = format!("{}:{}- ", i, win.name);
                write_out(&tab);
            }
        }

        // Status or prefix indicator
        if self.prefix_active {
            write_out(" \x1b[43;30m<PREFIX>\x1b[42;30m ");
        } else if let Some(ref msg) = self.status_message {
            let msg_fmt = format!(" ({}) ", msg);
            write_out(&msg_fmt);
        }

        let right_info = format!("   \"RP2040 SMP\" {:02}:{:02}\x1b[K\x1b[0m\x1b[u", mins, s);
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
                // 'c' or 'C': Create new window
                b'c' | b'C' => {
                    if self.windows.len() < MAX_WINDOWS {
                        let new_idx = self.windows.len();
                        let name = format!("sh{}", new_idx);
                        self.windows.push(TmuxWindow::new(&name));
                        self.active_idx = new_idx;
                        self.status_message = Some(format!("Created window {}", new_idx));
                        self.render(&mut write_out);
                    } else {
                        self.status_message = Some(String::from("Max 4 windows reached"));
                        self.render_status_bar(&mut write_out);
                    }
                    return;
                }
                // 'n' or 'N': Next window
                b'n' | b'N' => {
                    if !self.windows.is_empty() {
                        self.active_idx = (self.active_idx + 1) % self.windows.len();
                        self.status_message = None;
                        self.render(&mut write_out);
                    }
                    return;
                }
                // 'p' or 'P': Previous window
                b'p' | b'P' => {
                    if !self.windows.is_empty() {
                        self.active_idx = (self.active_idx + self.windows.len() - 1) % self.windows.len();
                        self.status_message = None;
                        self.render(&mut write_out);
                    }
                    return;
                }
                // '0'..='9': Select window by index
                b'0'..=b'9' => {
                    let idx = (byte - b'0') as usize;
                    if idx < self.windows.len() {
                        self.active_idx = idx;
                        self.status_message = None;
                        self.render(&mut write_out);
                    } else {
                        self.status_message = Some(format!("Window {} does not exist", idx));
                        self.render_status_bar(&mut write_out);
                    }
                    return;
                }
                // '&' or 'x' or 'k': Kill / close current window
                b'&' | b'x' | b'X' | b'k' | b'K' => {
                    if self.windows.len() > 1 {
                        self.windows.remove(self.active_idx);
                        if self.active_idx >= self.windows.len() {
                            self.active_idx = self.windows.len() - 1;
                        }
                        self.status_message = Some(String::from("Window closed"));
                        self.render(&mut write_out);
                    } else {
                        self.should_exit = true;
                    }
                    return;
                }
                // 'd' or 'D': Detach tmux session
                b'd' | b'D' => {
                    self.should_exit = true;
                    return;
                }
                // '?': Help overlay
                b'?' => {
                    if let Some(win) = self.windows.get_mut(self.active_idx) {
                        win.screen_lines.push(String::from("\x1b[1;36m=== Tmux Keybindings ===\x1b[0m"));
                        win.screen_lines.push(String::from("  Ctrl+B c   : Create new window"));
                        win.screen_lines.push(String::from("  Ctrl+B n/p : Next / Previous window"));
                        win.screen_lines.push(String::from("  Ctrl+B 0..3: Select window by number"));
                        win.screen_lines.push(String::from("  Ctrl+B &   : Close current window"));
                        win.screen_lines.push(String::from("  Ctrl+B d   : Detach back to main shell"));
                    }
                    self.render(&mut write_out);
                    return;
                }
                _ => {
                    self.status_message = Some(String::from("Unknown tmux command. Press '?' for help."));
                    self.render_status_bar(&mut write_out);
                    return;
                }
            }
        }

        // Standard terminal typing inside the active tmux window
        let active_idx = self.active_idx;
        if active_idx >= self.windows.len() {
            return;
        }

        match byte {
            // Enter key
            b'\r' | b'\n' => {
                let cmd_line = self.windows[active_idx].input_buffer.trim().to_string();
                self.windows[active_idx].input_buffer.clear();

                if cmd_line == "exit" || cmd_line == "quit" {
                    if self.windows.len() > 1 {
                        self.windows.remove(active_idx);
                        if self.active_idx >= self.windows.len() {
                            self.active_idx = self.windows.len() - 1;
                        }
                        self.render(&mut write_out);
                        return;
                    } else {
                        self.should_exit = true;
                        return;
                    }
                }

                if cmd_line == "clear" {
                    self.windows[active_idx].screen_lines.clear();
                    self.render(&mut write_out);
                    return;
                }

                let prompt_line = format!("\x1b[1;32mroot@pico\x1b[0m:\x1b[1;34m[tmux]\x1b[0m# {}", cmd_line);
                self.windows[active_idx].screen_lines.push(prompt_line);

                if !cmd_line.is_empty() {
                    self.windows[active_idx].history.push(cmd_line.clone());

                    // Execute command and capture output into window screen
                    let mut cmd_output: Vec<String> = Vec::new();
                    {
                        let mut ctx = CommandContext {
                            output: &mut |s| {
                                for line in s.split("\r\n") {
                                    if !line.is_empty() {
                                        cmd_output.push(String::from(line));
                                    }
                                }
                            },
                        };
                        execute_command(&cmd_line, &mut ctx);
                    }

                    self.windows[active_idx].screen_lines.extend(cmd_output);
                }

                // Trim buffer to prevent memory overflow
                while self.windows[active_idx].screen_lines.len() > MAX_SCREEN_LINES {
                    self.windows[active_idx].screen_lines.remove(0);
                }

                self.render(&mut write_out);
            }
            // Backspace
            0x08 | 0x7F => {
                if !self.windows[active_idx].input_buffer.is_empty() {
                    self.windows[active_idx].input_buffer.pop();
                    write_out("\x08 \x08");
                }
            }
            // Ctrl+C
            0x03 => {
                self.windows[active_idx].input_buffer.clear();
                self.windows[active_idx].screen_lines.push(String::from("^C"));
                self.render(&mut write_out);
            }
            // Ctrl+L
            0x0C => {
                self.windows[active_idx].screen_lines.clear();
                self.render(&mut write_out);
            }
            // Printable ASCII
            32..=126 => {
                let ch = byte as char;
                self.windows[active_idx].input_buffer.push(ch);
                let mut b = [0u8; 4];
                let s = ch.encode_utf8(&mut b);
                write_out(s);
            }
            _ => {}
        }
    }
}
