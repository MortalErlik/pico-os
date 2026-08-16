//! Neofetch / System Information Display for Pico OS

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;
use crate::mm;
use crate::task;

const LOGO: [&str; 11] = [
    "\x1b[1;35m   /\\_/\\   \x1b[0m",
    "\x1b[1;36m  ( o.o )  \x1b[0m",
    "\x1b[1;35m   > ^ <   \x1b[0m",
    "\x1b[1;33m  /  ~  \\  \x1b[0m",
    "\x1b[1;33m /|     |\\ \x1b[0m",
    "\x1b[1;33m(_|     |_)\x1b[0m",
    "\x1b[0;90m  (_____)  \x1b[0m",
    "           ",
    "           ",
    "           ",
    "           ",
];

pub fn render_fetch<F: FnMut(&str)>(mut write_out: F) {
    let ticks = task::get_uptime_ticks();
    let total_secs = ticks / 1000;
    let hours = total_secs / 3600;
    let mins = (total_secs % 3600) / 60;
    let secs = total_secs % 60;

    let mem = mm::get_stats();
    let mem_used_k = mem.used_bytes / 1024;
    let mem_total_k = mem.total_bytes / 1024;

    let (swap_used, swap_total) = mm::get_swap_usage();
    let swap_used_k = swap_used / 1024;
    let swap_total_k = swap_total / 1024;

    let tasks = task::get_tasks();
    let total_tasks = tasks.len();
    let cpu0_tasks = tasks.iter().filter(|t| t.core == 0).count();
    let cpu1_tasks = tasks.iter().filter(|t| t.core == 1).count();

    let mut info_lines: Vec<String> = Vec::new();
    info_lines.push(String::from("\x1b[1;36mroot\x1b[0m@\x1b[1;36mpico\x1b[0m"));
    info_lines.push(String::from("\x1b[0;90m-----------------------------------\x1b[0m"));
    info_lines.push(String::from("\x1b[1;33mOS\x1b[0m:      Pico-OS Dual-Core SMP (v0.1.0)"));
    info_lines.push(String::from("\x1b[1;33mHost\x1b[0m:    Raspberry Pi Pico (RP2040 Cortex-M0+)"));
    info_lines.push(String::from("\x1b[1;33mKernel\x1b[0m:  6.1.0-picos-smp (125 MHz Dual-Core)"));
    info_lines.push(format!("\x1b[1;33mUptime\x1b[0m:  {:02}:{:02}:{:02}", hours, mins, secs));
    info_lines.push(format!("\x1b[1;33mTasks\x1b[0m:   {} total (CPU0: {}, CPU1: {})", total_tasks, cpu0_tasks, cpu1_tasks));
    info_lines.push(format!(
        "\x1b[1;33mMemory\x1b[0m:  {}K / {}K (Swap: {}K / {}K)",
        mem_used_k, mem_total_k, swap_used_k, swap_total_k
    ));
    info_lines.push(String::from("\x1b[1;33mStorage\x1b[0m: VFS 256K | Raw Disk 1.4MB | Swap 128K"));
    info_lines.push(String::from("\x1b[1;33mShell\x1b[0m:   picos-sh v1.0"));
    info_lines.push(String::from("\x1b[1;33mGuard\x1b[0m:   95% RAM OOM-Protection Active"));

    let max_lines = core::cmp::max(LOGO.len(), info_lines.len());

    for i in 0..max_lines {
        let logo_part = if i < LOGO.len() {
            LOGO[i]
        } else {
            "              "
        };

        let info_part = if i < info_lines.len() {
            &info_lines[i]
        } else {
            ""
        };

        let line = format!("{}   {}\r\n", logo_part, info_part);
        write_out(&line);
    }

    // Print ANSI 16-Color Palette Bars
    write_out("\r\n                 \x1b[40m   \x1b[41m   \x1b[42m   \x1b[43m   \x1b[44m   \x1b[45m   \x1b[46m   \x1b[47m   \x1b[0m\r\n");
    write_out("                 \x1b[100m   \x1b[101m   \x1b[102m   \x1b[103m   \x1b[104m   \x1b[105m   \x1b[106m   \x1b[107m   \x1b[0m\r\n\r\n");
}
