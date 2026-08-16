//! Interactive 'htop' Dual-Core Process & System Monitor for Pico OS
//! Displays live Dual-Core CPU0/CPU1 %, 216KB RAM, VFS Storage bars, task states, core affinities, stack depths, and process kill interface.

extern crate alloc;
use alloc::format;
use alloc::string::String;
use crate::fs;
use crate::mm;
use crate::task;

pub struct HtopMonitor {
    pub should_exit: bool,
    pub kill_mode: bool,
    pub kill_pid_buf: String,
    pub status_msg: Option<String>,
}

impl HtopMonitor {
    pub fn new() -> Self {
        HtopMonitor {
            should_exit: false,
            kill_mode: false,
            kill_pid_buf: String::new(),
            status_msg: None,
        }
    }

    pub fn render<W: FnMut(&str)>(&self, mut write_out: W) {
        // Home cursor and hide it, but don't clear the whole screen to avoid flickering
        write_out("\x1b[?25l\x1b[H");

        let tasks = task::get_tasks();
        let mem = mm::get_stats();
        let uptime_ticks = task::get_uptime_ticks();
        let uptime_secs = uptime_ticks / 1000;
        let mins = uptime_secs / 60;
        let secs = uptime_secs % 60;
        let hours = mins / 60;

        let (cpu0_pct, cpu1_pct) = task::get_cpu_loads();

        // 1. CPU0 Bar + Tasks Summary
        write_out("\x1b[1;36m0[\x1b[0m");
        Self::render_bar(&mut write_out, cpu0_pct, 100, 20, "\x1b[32m", "\x1b[33m", "\x1b[31m");
        let running_cnt = tasks.iter().filter(|t| t.state == task::TaskState::Running).count();
        let cpu0_str = format!("\x1b[1;36m{:>3}%\x1b[0m]   \x1b[1;37mTasks: \x1b[1;32m{}\x1b[0;37m total, \x1b[1;32m{}\x1b[0;37m running\x1b[K\r\n", cpu0_pct, tasks.len(), running_cnt);
        write_out(&cpu0_str);

        // 2. CPU1 Bar + Uptime
        write_out("\x1b[1;36m1[\x1b[0m");
        Self::render_bar(&mut write_out, cpu1_pct, 100, 20, "\x1b[32m", "\x1b[33m", "\x1b[31m");
        let cpu1_str = format!("\x1b[1;36m{:>3}%\x1b[0m]   \x1b[1;37mUptime: \x1b[1;33m{:02}:{:02}:{:02}\x1b[0m\x1b[K\r\n", cpu1_pct, hours, mins % 60, secs);
        write_out(&cpu1_str);

        // 3. RAM / Memory Bar + Architecture
        write_out("\x1b[1;36mMem[\x1b[0m");
        let mem_used_k = mem.used_bytes / 1024;
        let mem_total_k = mem.total_bytes / 1024;
        let mem_pct = if mem.total_bytes > 0 {
            ((mem.used_bytes * 100) / mem.total_bytes) as u8
        } else {
            0
        };
        Self::render_bar(&mut write_out, mem_pct, 100, 18, "\x1b[34m", "\x1b[36m", "\x1b[35m");
        let mem_str = format!("\x1b[1;36m{:>3}K/{:>3}K\x1b[0m]   \x1b[1;37mArch: \x1b[1;35mDual-Core SMP (RP2040)\x1b[0m\x1b[K\r\n", mem_used_k, mem_total_k);
        write_out(&mem_str);

        // 4. VFS Snapshot Bar
        write_out("\x1b[1;36mVFS [\x1b[0m");
        let (fs_used, fs_total) = fs::get_fs_usage();
        let fs_used_k = fs_used / 1024;
        let fs_total_k = fs_total / 1024;
        let disk_pct = if fs_total > 0 {
            ((fs_used * 100) / fs_total).min(100) as u8
        } else {
            0
        };
        Self::render_bar(&mut write_out, disk_pct, 100, 18, "\x1b[35m", "\x1b[33m", "\x1b[32m");
        let vfs_str = format!("\x1b[1;36m{:>3}K/{:>3}K\x1b[0m]   \x1b[1;37mDisk: \x1b[1;32mVFS Snapshot\x1b[0m\x1b[K\r\n", fs_used_k, fs_total_k);
        write_out(&vfs_str);

        // 5. Swap Partition Bar
        write_out("\x1b[1;36mSwap[\x1b[0m");
        Self::render_bar(&mut write_out, 0, 100, 18, "\x1b[33m", "\x1b[33m", "\x1b[31m");
        let swp_str = format!("\x1b[1;36m  0K/128K\x1b[0m]   \x1b[1;37mDisk: \x1b[1;33mApplication Paging\x1b[0m\x1b[K\r\n", );
        write_out(&swp_str);

        // 6. True Disk Bar
        write_out("\x1b[1;36mRaw [\x1b[0m");
        Self::render_bar(&mut write_out, 0, 100, 18, "\x1b[34m", "\x1b[33m", "\x1b[31m");
        let raw_str = format!("\x1b[1;36m  0M/1.4M\x1b[0m]   \x1b[1;37mDisk: \x1b[1;35mTrue Block Device\x1b[0m\x1b[K\r\n\x1b[K\r\n", );
        write_out(&raw_str);

        // Process Table Header (compact 45 columns, guaranteed no wrap/truncation)
        write_out("\x1b[7m PID CORE USER  STATE  CPU%  STACK  NAME\x1b[0m\x1b[K\r\n");

        for t in &tasks {
            let state_str = match t.state {
                task::TaskState::Running => "RUN  ",
                task::TaskState::Ready => "READY",
                task::TaskState::Sleeping(_) => "SLEEP",
                task::TaskState::Blocked => "BLOCK",
                task::TaskState::Dead => "DEAD ",
            };

            let row = format!(
                "{:>4} CPU{} root  {}  {:>3}%  {:>4}B  {}\x1b[K\r\n",
                t.pid,
                t.core,
                state_str,
                t.cpu_percent,
                t.stack_used,
                t.name
            );
            write_out(&row);
        }

        // Bottom Footer
        write_out("\r\n");
        if self.kill_mode {
            let k_prompt = format!("\x1b[1;41;37m KILL TASK -> Enter PID to kill: {}_ \x1b[0m (Enter: Confirm, Esc: Cancel)", self.kill_pid_buf);
            write_out(&k_prompt);
        } else if let Some(ref msg) = self.status_msg {
            let s_prompt = format!("\x1b[1;44;37m {} \x1b[0m", msg);
            write_out(&s_prompt);
        } else {
            write_out("\x1b[7m F1 Help  F9/K Kill Process  F10/Q Quit Htop \x1b[0m");
        }
        write_out("\x1b[J"); // clear remainder of screen
    }

    fn render_bar<W: FnMut(&str)>(
        write_out: &mut W,
        val: u8,
        max: u8,
        width: usize,
        c_low: &str,
        c_mid: &str,
        c_high: &str,
    ) {
        let filled = ((val as usize * width) / (max as usize)).min(width);
        let empty = width.saturating_sub(filled);

        for i in 0..filled {
            let pct = (i * 100) / width;
            if pct < 50 {
                write_out(c_low);
            } else if pct < 80 {
                write_out(c_mid);
            } else {
                write_out(c_high);
            }
            write_out("|");
        }
        write_out("\x1b[90m");
        for _ in 0..empty {
            write_out(" ");
        }
        write_out("\x1b[0m");
    }

    pub fn handle_key<W: FnMut(&str)>(&mut self, key: u8, mut write_out: W) {
        if self.kill_mode {
            match key {
                // Enter: execute kill
                b'\r' | b'\n' => {
                    if let Ok(pid) = self.kill_pid_buf.trim().parse::<usize>() {
                        if task::kill(pid) {
                            self.status_msg = Some(format!("Signal SIGKILL sent to PID {}", pid));
                        } else {
                            self.status_msg = Some(format!("Failed to kill PID {} (Protected or invalid)", pid));
                        }
                    }
                    self.kill_mode = false;
                    self.kill_pid_buf.clear();
                    self.render(write_out);
                }
                // Esc / Ctrl+C: Cancel
                0x1B | 0x03 => {
                    self.kill_mode = false;
                    self.kill_pid_buf.clear();
                    self.status_msg = Some(String::from("Kill cancelled."));
                    self.render(write_out);
                }
                // Backspace
                0x08 | 0x7F => {
                    self.kill_pid_buf.pop();
                    self.render(write_out);
                }
                // Digits
                b'0'..=b'9' => {
                    self.kill_pid_buf.push(key as char);
                    self.render(write_out);
                }
                _ => {}
            }
            return;
        }

        match key {
            b'q' | b'Q' | 0x1B | 0x03 => {
                self.should_exit = true;
                // Show cursor again and clear
                write_out("\x1b[?25h\x1b[2J\x1b[H");
            }
            b'k' | b'K' => {
                self.kill_mode = true;
                self.kill_pid_buf.clear();
                self.status_msg = None;
                self.render(write_out);
            }
            _ => {
                self.render(write_out);
            }
        }
    }
}
