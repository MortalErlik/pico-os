//! Interactive Unix-like Shell for Pico OS

pub mod commands;

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::editor::NanoEditor;
use crate::fs;
use crate::htop::HtopMonitor;
use commands::{execute_command, CommandContext};

pub enum ShellMode {
    LineInput,
    Nano(NanoEditor),
    Htop(HtopMonitor),
}

pub struct Shell {
    pub mode: ShellMode,
    pub input_buffer: String,
    pub history: Vec<String>,
    pub ansi_state: usize,
}

impl Shell {
    pub fn new() -> Self {
        Shell {
            mode: ShellMode::LineInput,
            input_buffer: String::new(),
            history: Vec::new(),
            ansi_state: 0,
        }
    }

    pub fn print_banner<W: FnMut(&str)>(&self, mut write_out: W) {
        write_out("\r\n\x1b[1;32m");
        write_out("  ____  _            ____   ____  \r\n");
        write_out(" |  _ \\(_) ___ ___  / __ \\ / ___| \r\n");
        write_out(" | |_) | |/ __/ _ \\| |  | |\\___ \\ \r\n");
        write_out(" |  __/| | (_| (_) | |__| | ___) |\r\n");
        write_out(" |_|   |_|\\___\\___/ \\____/ |____/ \r\n");
        write_out("\x1b[0m");
        write_out("\x1b[1;37m Custom Bare-Metal OS in Rust & Assembly on RP2040\x1b[0m\r\n");
        write_out("\x1b[0;90m Developed for Raspberry Pi Pico + ESP8266 + SSD1306 OLED\x1b[0m\r\n");
        write_out("\x1b[0;33m Type 'help' to see available commands or 'nano readme.txt' to edit.\x1b[0m\r\n\r\n");
        self.print_prompt(write_out);
    }

    pub fn print_prompt<W: FnMut(&str)>(&self, mut write_out: W) {
        let cwd = fs::with_fs(|fs| fs.get_cwd().to_string());
        let prompt = format!("\x1b[1;32mroot@pico\x1b[0m:\x1b[1;34m{}\x1b[0m# ", cwd);
        write_out(&prompt);
    }

    pub fn tick<W: FnMut(&str)>(&mut self, mut write_out: W) {
        if let ShellMode::Htop(ref mut htop) = self.mode {
            htop.render(&mut write_out);
        }
    }

    pub fn handle_byte<W: FnMut(&str)>(&mut self, byte: u8, mut write_out: W) {
        // Handle ANSI Escape Sequences (like arrow keys)
        if self.ansi_state == 1 {
            if byte == b'[' {
                self.ansi_state = 2;
                return;
            } else {
                self.ansi_state = 0;
            }
        } else if self.ansi_state == 2 {
            self.ansi_state = 0;
            match &mut self.mode {
                ShellMode::Nano(ref mut editor) => {
                    editor.handle_ansi_arrow(byte, &mut write_out);
                    return;
                }
                ShellMode::Htop(ref mut htop) => {
                    htop.handle_key(byte, &mut write_out);
                    return;
                }
                ShellMode::LineInput => {
                    return;
                }
            }
        }

        if byte == 0x1B {
            self.ansi_state = 1;
            return;
        }

        match &mut self.mode {
            ShellMode::Nano(ref mut editor) => {
                editor.handle_key(byte, &mut write_out);
                if editor.should_exit {
                    self.mode = ShellMode::LineInput;
                    write_out("\x1b[?25h\x1b[2J\x1b[H");
                    self.print_prompt(&mut write_out);
                }
            }
            ShellMode::Htop(ref mut htop) => {
                htop.handle_key(byte, &mut write_out);
                if htop.should_exit {
                    self.mode = ShellMode::LineInput;
                    write_out("\x1b[?25h\x1b[2J\x1b[H");
                    self.print_prompt(&mut write_out);
                }
            }
            ShellMode::LineInput => {
                match byte {
                    // Enter (\r or \n)
                    b'\r' | b'\n' => {
                        write_out("\r\n");
                        let cmd_line = self.input_buffer.trim().to_string();
                        self.input_buffer.clear();

                        if !cmd_line.is_empty() {
                            self.history.push(cmd_line.clone());

                            // Check special full-screen interactive commands
                            if cmd_line.starts_with("nano") {
                                let parts: Vec<&str> = cmd_line.split_whitespace().collect();
                                let target_file = parts.get(1).copied().unwrap_or("untitled.txt");
                                let editor = NanoEditor::new(target_file);
                                editor.render(&mut write_out);
                                self.mode = ShellMode::Nano(editor);
                                return;
                            } else if cmd_line == "htop" || cmd_line == "top" {
                                let htop = HtopMonitor::new();
                                htop.render(&mut write_out);
                                self.mode = ShellMode::Htop(htop);
                                return;
                            }

                            let mut ctx = CommandContext {
                                output: &mut write_out,
                            };
                            execute_command(&cmd_line, &mut ctx);
                        }

                        self.print_prompt(write_out);
                    }
                    // Backspace (0x08 or 0x7F)
                    0x08 | 0x7F => {
                        if !self.input_buffer.is_empty() {
                            self.input_buffer.pop();
                            write_out("\x08 \x08");
                        }
                    }
                    // Ctrl+C
                    0x03 => {
                        self.input_buffer.clear();
                        write_out("^C\r\n");
                        self.print_prompt(write_out);
                    }
                    // Ctrl+L
                    0x0C => {
                        write_out("\x1b[2J\x1b[H");
                        self.print_prompt(&mut write_out);
                        write_out(&self.input_buffer);
                    }
                    // Printable ASCII characters
                    32..=126 => {
                        let ch = byte as char;
                        self.input_buffer.push(ch);
                        let mut b = [0u8; 4];
                        let s = ch.encode_utf8(&mut b);
                        write_out(s);
                    }
                    _ => {}
                }
            }
        }
    }
}
