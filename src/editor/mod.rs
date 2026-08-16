//! Interactive 'nano' Text Editor for Pico OS
//! Implements a full-screen ANSI terminal text editor with buffer management,
//! arrow key navigation, line editing, and file save/exit capabilities.

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::fs;

pub struct NanoEditor {
    pub filename: String,
    pub lines: Vec<String>,
    pub cursor_x: usize,
    pub cursor_y: usize,
    pub scroll_y: usize,
    pub modified: bool,
    pub status_message: String,
    pub should_exit: bool,
    pub rows: usize,
    pub cols: usize,
}

impl NanoEditor {
    pub fn new(filename: &str) -> Self {
        let content_res = fs::with_fs(|fs| fs.read_file(filename));
        let mut lines = Vec::new();

        if let Ok(bytes) = content_res {
            if let Ok(text) = core::str::from_utf8(&bytes) {
                for line in text.lines() {
                    lines.push(line.to_string());
                }
            }
        }

        if lines.is_empty() {
            lines.push(String::new());
        }

        NanoEditor {
            filename: filename.to_string(),
            lines,
            cursor_x: 0,
            cursor_y: 0,
            scroll_y: 0,
            modified: false,
            status_message: String::from("^O WriteOut   ^X Exit   ^K Cut Line"),
            should_exit: false,
            rows: 24,
            cols: 80,
        }
    }

    pub fn render<W: FnMut(&str)>(&self, mut write_out: W) {
        // Hide cursor during render
        write_out("\x1b[?25l");
        // Clear screen and go to top-left
        write_out("\x1b[2J\x1b[H");

        // Top Status Header (Inverse video)
        let mod_flag = if self.modified { " [Modified]" } else { "" };
        let header = format!("  Pico Nano 1.0               File: {}{}", self.filename, mod_flag);
        write_out("\x1b[7m");
        write_out(&header);
        // Fill header row to width
        let pad = self.cols.saturating_sub(header.len());
        for _ in 0..pad {
            write_out(" ");
        }
        write_out("\x1b[0m\r\n");

        // Text Body
        let body_rows = self.rows.saturating_sub(2);
        for row in 0..body_rows {
            let line_idx = self.scroll_y + row;
            if line_idx < self.lines.len() {
                let line = &self.lines[line_idx];
                write_out(line);
            } else {
                write_out("\x1b[90m~\x1b[0m");
            }
            write_out("\r\n");
        }

        // Bottom Shortcut Bar (Inverse video)
        write_out("\x1b[7m");
        write_out(&self.status_message);
        let pad_b = self.cols.saturating_sub(self.status_message.len());
        for _ in 0..pad_b {
            write_out(" ");
        }
        write_out("\x1b[0m");

        // Position Cursor
        let screen_y = (self.cursor_y.saturating_sub(self.scroll_y) + 2).min(self.rows);
        let screen_x = (self.cursor_x + 1).min(self.cols);
        let cursor_cmd = format!("\x1b[{};{}H\x1b[?25h", screen_y, screen_x);
        write_out(&cursor_cmd);
    }

    pub fn handle_key<W: FnMut(&str)>(&mut self, key: u8, write_out: W) {
        match key {
            // Ctrl+X: Exit
            0x18 => {
                self.should_exit = true;
            }
            // Ctrl+O: Save File
            0x0F => {
                self.save();
                self.status_message = format!("[ Wrote {} lines to {} ]", self.lines.len(), self.filename);
                self.render(write_out);
            }
            // Ctrl+K: Cut line
            0x0B => {
                if self.lines.len() > 1 {
                    self.lines.remove(self.cursor_y);
                    if self.cursor_y >= self.lines.len() {
                        self.cursor_y = self.lines.len() - 1;
                    }
                    self.cursor_x = 0;
                    self.modified = true;
                } else {
                    self.lines[0].clear();
                    self.cursor_x = 0;
                    self.modified = true;
                }
                self.render(write_out);
            }
            // Backspace (0x08 or 0x7F)
            0x08 | 0x7F => {
                if self.cursor_x > 0 {
                    let line = &mut self.lines[self.cursor_y];
                    if self.cursor_x <= line.len() {
                        line.remove(self.cursor_x - 1);
                        self.cursor_x -= 1;
                        self.modified = true;
                    }
                } else if self.cursor_y > 0 {
                    // Merge with previous line
                    let current_line = self.lines.remove(self.cursor_y);
                    self.cursor_y -= 1;
                    let prev_line = &mut self.lines[self.cursor_y];
                    self.cursor_x = prev_line.len();
                    prev_line.push_str(&current_line);
                    self.modified = true;
                }
                self.render(write_out);
            }
            // Enter (\r or \n)
            b'\r' | b'\n' => {
                let current_line = &mut self.lines[self.cursor_y];
                let remainder = if self.cursor_x < current_line.len() {
                    let rem = current_line[self.cursor_x..].to_string();
                    current_line.truncate(self.cursor_x);
                    rem
                } else {
                    String::new()
                };

                self.cursor_y += 1;
                self.lines.insert(self.cursor_y, remainder);
                self.cursor_x = 0;
                self.modified = true;

                let body_rows = self.rows.saturating_sub(2);
                if self.cursor_y >= self.scroll_y + body_rows {
                    self.scroll_y += 1;
                }

                self.render(write_out);
            }
            // Printable Characters (ASCII 32 to 126)
            32..=126 => {
                let ch = key as char;
                let line = &mut self.lines[self.cursor_y];
                if self.cursor_x >= line.len() {
                    line.push(ch);
                } else {
                    line.insert(self.cursor_x, ch);
                }
                self.cursor_x += 1;
                self.modified = true;
                self.render(write_out);
            }
            _ => {}
        }
    }

    pub fn handle_ansi_arrow<W: FnMut(&str)>(&mut self, arrow: u8, write_out: W) {
        match arrow {
            b'A' => { // Up
                if self.cursor_y > 0 {
                    self.cursor_y -= 1;
                    if self.cursor_y < self.scroll_y {
                        self.scroll_y = self.cursor_y;
                    }
                    self.cursor_x = self.cursor_x.min(self.lines[self.cursor_y].len());
                }
            }
            b'B' => { // Down
                if self.cursor_y + 1 < self.lines.len() {
                    self.cursor_y += 1;
                    let body_rows = self.rows.saturating_sub(2);
                    if self.cursor_y >= self.scroll_y + body_rows {
                        self.scroll_y = self.cursor_y - body_rows + 1;
                    }
                    self.cursor_x = self.cursor_x.min(self.lines[self.cursor_y].len());
                }
            }
            b'C' => { // Right
                if self.cursor_x < self.lines[self.cursor_y].len() {
                    self.cursor_x += 1;
                } else if self.cursor_y + 1 < self.lines.len() {
                    self.cursor_y += 1;
                    self.cursor_x = 0;
                }
            }
            b'D' => { // Left
                if self.cursor_x > 0 {
                    self.cursor_x -= 1;
                } else if self.cursor_y > 0 {
                    self.cursor_y -= 1;
                    self.cursor_x = self.lines[self.cursor_y].len();
                }
            }
            _ => {}
        }
        self.render(write_out);
    }

    pub fn save(&mut self) {
        let mut buffer = Vec::new();
        for line in &self.lines {
            buffer.extend_from_slice(line.as_bytes());
            buffer.push(b'\n');
        }
        let _ = fs::with_fs(|fs| fs.write_file(&self.filename, &buffer));
        self.modified = false;
    }
}
