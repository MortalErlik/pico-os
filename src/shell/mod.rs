//! Interactive Unix-like Shell for Pico OS

pub mod commands;
pub mod fetch;

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::editor::NanoEditor;
use crate::fs;
use crate::htop::HtopMonitor;
use crate::tmux::TmuxManager;
use commands::{execute_command, CommandContext};

pub enum ShellMode {
    LineInput,
    Nano(NanoEditor),
    Htop(HtopMonitor),
    Calc(crate::calc::CalcContext),
    Tmux(TmuxManager),
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
        write_out("\x1b[1;37m Custom Bare-Metal OS in Rust & Assembly on RP2040 Dual-Core SMP\x1b[0m\r\n");
        write_out("\x1b[0;90m Developed for Raspberry Pi Pico + ESP8266 + SSD1306 OLED\x1b[0m\r\n");
        write_out("\x1b[1;36m Apps & Tools: \x1b[1;33mfetch\x1b[0m | \x1b[1;33mtmux\x1b[0m | \x1b[1;33mhtop\x1b[0m | \x1b[1;33mcalc\x1b[0m | \x1b[1;33mnano\x1b[0m | \x1b[1;33mservice list\x1b[0m\r\n");
        write_out("\x1b[0;32m Type '\x1b[1;32mhelp\x1b[0;32m' for command reference or '\x1b[1;32mtmux help\x1b[0;32m' for split-screen guide.\x1b[0m\r\n\r\n");
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
                ShellMode::LineInput | ShellMode::Calc(_) | ShellMode::Tmux(_) => {
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
            ShellMode::Tmux(ref mut tmux) => {
                tmux.handle_key(byte, &mut write_out);
                if tmux.should_exit {
                    self.mode = ShellMode::LineInput;
                    write_out("\x1b[?25h\x1b[2J\x1b[H");
                    self.print_prompt(&mut write_out);
                }
            }
            ShellMode::Calc(ref mut calc_ctx) => {
                match byte {
                    // Enter (\r or \n)
                    b'\r' | b'\n' => {
                        write_out("\r\n");
                        let expr = self.input_buffer.trim().to_string();
                        self.input_buffer.clear();

                        if !expr.is_empty() {
                            match calc_ctx.eval(&expr) {
                                Ok(crate::calc::CalcOutput::Exit) => {
                                    self.mode = ShellMode::LineInput;
                                    self.print_prompt(&mut write_out);
                                    return;
                                }
                                Ok(crate::calc::CalcOutput::Help) => {
                                    write_out("\x1b[1;36mPico OS Calculator Functions & Syntax:\x1b[0m\r\n");
                                    write_out("  Operators : + - * / % ^ ( )\r\n");
                                    write_out("  Functions : sqrt(x), abs(x), pow(a,b), min(a,b), max(a,b), round(x), floor(x), ceil(x)\r\n");
                                    write_out("  Variables : x = 42, y = x * 2, ans (last result), pi, e\r\n");
                                    write_out("  Commands  : 'vars' (list variables), 'help', 'exit' or 'quit'\r\n");
                                }
                                Ok(crate::calc::CalcOutput::VarList(vars)) => {
                                    write_out("\x1b[1;36mVariables:\x1b[0m\r\n");
                                    for v in vars {
                                        write_out("  ");
                                        write_out(&v);
                                        write_out("\r\n");
                                    }
                                }
                                Ok(crate::calc::CalcOutput::Value(val)) => {
                                    let s = format!("= \x1b[1;32m{}\x1b[0m\r\n", crate::calc::format_num(val));
                                    write_out(&s);
                                }
                                Ok(crate::calc::CalcOutput::Assignment(name, val)) => {
                                    let s = format!("{} = \x1b[1;32m{}\x1b[0m\r\n", name, crate::calc::format_num(val));
                                    write_out(&s);
                                }
                                Ok(crate::calc::CalcOutput::Empty) => {}
                                Err(e) => {
                                    let s = format!("\x1b[31mError: {}\x1b[0m\r\n", e);
                                    write_out(&s);
                                }
                            }
                        }
                        write_out("\x1b[1;33mcalc>\x1b[0m ");
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
                        self.mode = ShellMode::LineInput;
                        write_out("^C\r\n");
                        self.print_prompt(write_out);
                    }
                    // Ctrl+L
                    0x0C => {
                        write_out("\x1b[2J\x1b[H\x1b[1;33mcalc>\x1b[0m ");
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
                            } else if cmd_line == "calc" || cmd_line == "bc" {
                                write_out("\x1b[1;36m=== Pico OS Math Calculator REPL ===\x1b[0m\r\n");
                                write_out("\x1b[0;90mOperators: +, -, *, /, %, ^ | Funcs: sqrt, abs, pow, min, max, round, pi, e, ans\x1b[0m\r\n");
                                write_out("\x1b[0;33mType expressions (e.g. x = 10, sqrt(x) * 2), 'vars', or 'exit' to leave.\x1b[0m\r\n\r\n");
                                write_out("\x1b[1;33mcalc>\x1b[0m ");
                                self.mode = ShellMode::Calc(crate::calc::CalcContext::new());
                                return;
                            } else if cmd_line == "tmux" {
                                let mut tmux = TmuxManager::new();
                                tmux.render(&mut write_out);
                                self.mode = ShellMode::Tmux(tmux);
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
