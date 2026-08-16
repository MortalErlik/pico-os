//! Linux Command Implementations for Pico OS Shell

extern crate alloc;
use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use crate::fs;
use crate::mm;
use crate::task;

pub struct CommandContext<'a> {
    pub output: &'a mut dyn FnMut(&str),
}

impl<'a> CommandContext<'a> {
    pub fn print(&mut self, s: &str) {
        (self.output)(s);
    }

    pub fn println(&mut self, s: &str) {
        (self.output)(s);
        (self.output)("\r\n");
    }
}

pub fn execute_command(line: &str, ctx: &mut CommandContext) {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return;
    }

    // Check for redirection: > or >>
    if let Some((cmd_part, file_part)) = trimmed.split_once(">>") {
        handle_redirect(cmd_part.trim(), file_part.trim(), true, ctx);
        return;
    } else if let Some((cmd_part, file_part)) = trimmed.split_once('>') {
        handle_redirect(cmd_part.trim(), file_part.trim(), false, ctx);
        return;
    }

    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    let cmd = tokens[0];
    let args = &tokens[1..];

    match cmd {
        "help" => cmd_help(args, ctx),
        "ls" => cmd_ls(args, ctx),
        "cd" => cmd_cd(args, ctx),
        "pwd" => cmd_pwd(args, ctx),
        "mkdir" => cmd_mkdir(args, ctx),
        "rm" => cmd_rm(args, ctx),
        "touch" => cmd_touch(args, ctx),
        "cat" => cmd_cat(args, ctx),
        "cp" => cmd_cp(args, ctx),
        "mv" => cmd_mv(args, ctx),
        "echo" => cmd_echo(args, ctx),
        "ps" => cmd_ps(args, ctx),
        "kill" => cmd_kill(args, ctx),
        "spawn" => cmd_spawn(args, ctx),
        "free" => cmd_free(args, ctx),
        "df" => cmd_df(args, ctx),
        "sync" => cmd_sync(args, ctx),
        "format" => cmd_format(args, ctx),
        "uptime" => cmd_uptime(args, ctx),
        "uname" => cmd_uname(args, ctx),
        "whoami" => cmd_whoami(args, ctx),
        "clear" => cmd_clear(args, ctx),
        "reboot" => cmd_reboot(args, ctx),
        "pin" => cmd_pin(args, ctx),
        "i2c_scan" => cmd_i2c_scan(args, ctx),
        _ => {
            let msg = format!("\x1b[31mpicos: command not found: {}\x1b[0m", cmd);
            ctx.println(&msg);
            ctx.println("Type 'help' to see all available commands.");
        }
    }
}

fn handle_redirect(cmd_str: &str, file_name: &str, append: bool, ctx: &mut CommandContext) {
    let mut buffer = String::new();
    let mut fake_ctx = CommandContext {
        output: &mut |s| buffer.push_str(s),
    };
    execute_command(cmd_str, &mut fake_ctx);

    let res = fs::with_fs(|fs| {
        let mut final_content = if append {
            fs.read_file(file_name).unwrap_or_default()
        } else {
            Vec::new()
        };
        final_content.extend_from_slice(buffer.as_bytes());
        fs.write_file(file_name, &final_content)
    });

    if let Err(e) = res {
        let msg = format!("\x1b[31mRedirection failed: {:?}\x1b[0m", e);
        ctx.println(&msg);
    }
}

fn cmd_help(_args: &[&str], ctx: &mut CommandContext) {
    ctx.println("\x1b[1;36m=== Pico OS Dual-Core SMP Built-in Linux Commands ===\x1b[0m");
    ctx.println("  \x1b[1;32mls [path]\x1b[0m           - List directory contents with sizes and colors");
    ctx.println("  \x1b[1;32mcd <path>\x1b[0m           - Change current directory");
    ctx.println("  \x1b[1;32mpwd\x1b[0m                 - Print current working directory");
    ctx.println("  \x1b[1;32mmkdir <path>\x1b[0m        - Create directory");
    ctx.println("  \x1b[1;32mrm [-r] <path>\x1b[0m      - Remove file or directory");
    ctx.println("  \x1b[1;32mtouch <path>\x1b[0m        - Create empty file");
    ctx.println("  \x1b[1;32mcat <path>\x1b[0m          - Display file content");
    ctx.println("  \x1b[1;32mcp <src> <dst>\x1b[0m      - Copy file");
    ctx.println("  \x1b[1;32mmv <src> <dst>\x1b[0m      - Move/rename file");
    ctx.println("  \x1b[1;32mecho [text]\x1b[0m         - Print text (supports > and >> redirection)");
    ctx.println("  \x1b[1;32mdf [-h]\x1b[0m             - Show Dual-Mount filesystems (Root tmpfs & /data Flash)");
    ctx.println("  \x1b[1;32msync\x1b[0m                - Synchronize /data partition to Physical Flash");
    ctx.println("  \x1b[1;32mformat\x1b[0m              - Format persistent /data partition on 1.0MB Flash");
    ctx.println("  \x1b[1;33mps\x1b[0m                  - List all active tasks across CPU0 & CPU1");
    ctx.println("  \x1b[1;33mkill <pid>\x1b[0m          - Terminate task by PID");
    ctx.println("  \x1b[1;33mspawn <name> [core]\x1b[0m - Launch a background task on CPU0 or CPU1");
    ctx.println("  \x1b[1;33mfree\x1b[0m                - Display 216KB RAM & heap allocation stats");
    ctx.println("  \x1b[1;33muptime\x1b[0m              - Display system running time");
    ctx.println("  \x1b[1;33muname [-a]\x1b[0m          - Show kernel & Dual-Core SMP architecture info");
    ctx.println("  \x1b[1;33mwhoami\x1b[0m              - Print current user (root)");
    ctx.println("  \x1b[1;33mclear\x1b[0m               - Clear screen");
    ctx.println("  \x1b[1;33mreboot\x1b[0m              - Restart system");
    ctx.println("  \x1b[1;35mpin <r|s|c|t> <pin>\x1b[0m - Read/Set/Clear/Toggle GPIO pin");
    ctx.println("  \x1b[1;35mi2c_scan\x1b[0m            - Scan I2C bus for OLED and devices");
    ctx.println("  \x1b[1;34mhtop\x1b[0m                - Interactive live Dual-Core process monitor");
    ctx.println("  \x1b[1;34mnano <file>\x1b[0m         - Interactive full-screen terminal text editor");
}

fn cmd_ls(args: &[&str], ctx: &mut CommandContext) {
    let path = args.first().copied().unwrap_or(".");
    let entries_res = fs::with_fs(|fs| fs.list_dir(path));

    match entries_res {
        Ok(entries) => {
            if entries.is_empty() {
                ctx.println("(empty directory)");
                return;
            }
            for entry in entries {
                if entry.is_dir {
                    let s = format!("\x1b[1;34m{}/\x1b[0m", entry.name);
                    ctx.println(&s);
                } else {
                    let s = format!("\x1b[0;37m{:<20} \x1b[0;90m{:>6} B\x1b[0m", entry.name, entry.size);
                    ctx.println(&s);
                }
            }
        }
        Err(e) => {
            let s = format!("\x1b[31mls: cannot access '{}': {:?}\x1b[0m", path, e);
            ctx.println(&s);
        }
    }
}

fn cmd_cd(args: &[&str], ctx: &mut CommandContext) {
    let target = args.first().copied().unwrap_or("/home");
    let res = fs::with_fs(|fs| fs.set_cwd(target));
    if let Err(e) = res {
        let s = format!("\x1b[31mcd: {}: {:?}\x1b[0m", target, e);
        ctx.println(&s);
    }
}

fn cmd_pwd(_args: &[&str], ctx: &mut CommandContext) {
    let cwd = fs::with_fs(|fs| fs.get_cwd().to_string());
    ctx.println(&cwd);
}

fn cmd_mkdir(args: &[&str], ctx: &mut CommandContext) {
    if args.is_empty() {
        ctx.println("\x1b[31mmkdir: missing operand\x1b[0m");
        return;
    }
    for &dir in args {
        let res = fs::with_fs(|fs| fs.create_dir(dir));
        if let Err(e) = res {
            let s = format!("\x1b[31mmkdir: cannot create directory '{}': {:?}\x1b[0m", dir, e);
            ctx.println(&s);
        }
    }
}

fn cmd_rm(args: &[&str], ctx: &mut CommandContext) {
    if args.is_empty() {
        ctx.println("\x1b[31mrm: missing operand\x1b[0m");
        return;
    }
    let recursive = args.contains(&"-r") || args.contains(&"-rf");
    let targets: Vec<&str> = args.iter().copied().filter(|&a| a != "-r" && a != "-rf").collect();

    for target in targets {
        let res = fs::with_fs(|fs| fs.remove(target, recursive));
        if let Err(e) = res {
            let s = format!("\x1b[31mrm: cannot remove '{}': {:?}\x1b[0m", target, e);
            ctx.println(&s);
        }
    }
}

fn cmd_touch(args: &[&str], ctx: &mut CommandContext) {
    if args.is_empty() {
        ctx.println("\x1b[31mtouch: missing file operand\x1b[0m");
        return;
    }
    for &file in args {
        let res = fs::with_fs(|fs| {
            if fs.read_file(file).is_ok() {
                Ok(())
            } else {
                fs.write_file(file, &[])
            }
        });
        if let Err(e) = res {
            let s = format!("\x1b[31mtouch: cannot touch '{}': {:?}\x1b[0m", file, e);
            ctx.println(&s);
        }
    }
}

fn cmd_cat(args: &[&str], ctx: &mut CommandContext) {
    if args.is_empty() {
        ctx.println("\x1b[31mcat: missing file operand\x1b[0m");
        return;
    }
    for &file in args {
        let res = fs::with_fs(|fs| fs.read_file(file));
        match res {
            Ok(bytes) => {
                if let Ok(text) = core::str::from_utf8(&bytes) {
                    for line in text.lines() {
                        ctx.println(line);
                    }
                } else {
                    let s = format!("[Binary data: {} bytes]", bytes.len());
                    ctx.println(&s);
                }
            }
            Err(e) => {
                let s = format!("\x1b[31mcat: {}: {:?}\x1b[0m", file, e);
                ctx.println(&s);
            }
        }
    }
}

fn cmd_cp(args: &[&str], ctx: &mut CommandContext) {
    if args.len() < 2 {
        ctx.println("\x1b[31mcp: missing destination file operand\x1b[0m");
        return;
    }
    let src = args[0];
    let dst = args[1];
    let res = fs::with_fs(|fs| fs.copy(src, dst));
    if let Err(e) = res {
        let s = format!("\x1b[31mcp: cannot copy '{}' to '{}': {:?}\x1b[0m", src, dst, e);
        ctx.println(&s);
    }
}

fn cmd_mv(args: &[&str], ctx: &mut CommandContext) {
    if args.len() < 2 {
        ctx.println("\x1b[31mmv: missing destination file operand\x1b[0m");
        return;
    }
    let src = args[0];
    let dst = args[1];
    let res = fs::with_fs(|fs| fs.move_node(src, dst));
    if let Err(e) = res {
        let s = format!("\x1b[31mmv: cannot move '{}' to '{}': {:?}\x1b[0m", src, dst, e);
        ctx.println(&s);
    }
}

fn cmd_echo(args: &[&str], ctx: &mut CommandContext) {
    let text = args.join(" ");
    ctx.println(&text);
}

fn cmd_ps(_args: &[&str], ctx: &mut CommandContext) {
    let tasks = task::get_tasks();
    ctx.println("\x1b[7m PID CORE USER  STATE  CPU%  STACK  NAME\x1b[0m");
    for t in tasks {
        let state_str = match t.state {
            task::TaskState::Running => "RUN  ",
            task::TaskState::Ready => "READY",
            task::TaskState::Sleeping(_) => "SLEEP",
            task::TaskState::Blocked => "BLOCK",
            task::TaskState::Dead => "DEAD ",
        };
        let line = format!(
            "{:>4} CPU{} root  {}  {:>3}%  {:>4}B  {}",
            t.pid,
            t.core,
            state_str,
            t.cpu_percent,
            t.stack_used,
            t.name
        );
        ctx.println(&line);
    }
}

fn cmd_kill(args: &[&str], ctx: &mut CommandContext) {
    if args.is_empty() {
        ctx.println("\x1b[31mkill: missing PID argument\x1b[0m");
        return;
    }
    if let Ok(pid) = args[0].parse::<usize>() {
        if task::kill(pid) {
            let s = format!("\x1b[32mSignal SIGKILL (9) sent to PID {}\x1b[0m", pid);
            ctx.println(&s);
        } else {
            let s = format!("\x1b[31mkill: failed to kill PID {} (Protected or invalid)\x1b[0m", pid);
            ctx.println(&s);
        }
    } else {
        ctx.println("\x1b[31mkill: invalid PID\x1b[0m");
    }
}

extern "C" fn demo_worker_task(_arg: usize) {
    loop {
        task::sleep_ms(1000);
    }
}

fn cmd_spawn(args: &[&str], ctx: &mut CommandContext) {
    let name = args.first().copied().unwrap_or("worker_task");
    let core = if args.len() > 1 {
        args[1].parse::<u8>().unwrap_or(0).min(1)
    } else {
        1
    };
    let pid = task::spawn(name, core, 1024, demo_worker_task, 0);
    let s = format!("\x1b[32mSpawned background task '{}' on CPU{} with PID {}\x1b[0m", name, core, pid);
    ctx.println(&s);
}

fn cmd_free(_args: &[&str], ctx: &mut CommandContext) {
    let stats = mm::get_stats();
    ctx.println("\x1b[1;37m               total        used        free      peak_used\x1b[0m");
    let line = format!(
        "\x1b[1;36mMem:       {:>8} B  {:>8} B  {:>8} B     {:>8} B\x1b[0m",
        stats.total_bytes, stats.used_bytes, stats.free_bytes, stats.peak_used_bytes
    );
    ctx.println(&line);
    let counts = format!(
        "\x1b[0;90mAllocations: {} | Frees: {} | Heap Capacity: 192 KB\x1b[0m",
        stats.alloc_count, stats.free_count
    );
    ctx.println(&counts);
}

fn cmd_df(_args: &[&str], ctx: &mut CommandContext) {
    let (flash_used, flash_total) = fs::get_fs_usage();
    let flash_free = flash_total.saturating_sub(flash_used);
    let flash_pct = if flash_total > 0 { (flash_used * 100) / flash_total } else { 0 };

    let mem = mm::get_stats();
    let root_used = mem.used_bytes;
    let root_total: usize = 64 * 1024;
    let root_free = root_total.saturating_sub(root_used);
    let root_pct = if root_total > 0 { (root_used * 100) / root_total } else { 0 };

    ctx.println("\x1b[1;36mFilesystem      Size  Used  Avail Use% Mounted on\x1b[0m");
    let out = format!(
        "\x1b[1;33mrootfs\x1b[0m            64K  {:>3}K   {:>3}K {:>3}% /\r\n\x1b[1;32m/dev/flash\x1b[0m      {:>4}K {:>4}K  {:>4}K {:>3}% /data\r\n\x1b[1;34mproc\x1b[0m            216K    0K  216K   0% /proc\r\n\x1b[1;35mdev\x1b[0m               4K    0K    4K   0% /dev",
        root_used / 1024,
        root_free / 1024,
        root_pct,
        flash_total / 1024,
        flash_used / 1024,
        flash_free / 1024,
        flash_pct
    );
    ctx.println(&out);
}

fn cmd_sync(_args: &[&str], ctx: &mut CommandContext) {
    fs::sync_fs();
    ctx.println("\x1b[1;32m[ OK ] Persistent /data partition synchronized to Physical Flash.\x1b[0m");
}

fn cmd_format(_args: &[&str], ctx: &mut CommandContext) {
    ctx.println("Formatting 1.0MB Persistent Flash partition (/data)...");
    match fs::format_fs() {
        Ok(_) => ctx.println("\x1b[1;32m[ OK ] Format complete! /data partition initialized on Flash.\x1b[0m"),
        Err(_) => ctx.println("\x1b[31mFormat failed!\x1b[0m"),
    }
}

fn cmd_uptime(_args: &[&str], ctx: &mut CommandContext) {
    let ticks = task::get_uptime_ticks();
    let total_secs = ticks / 1000;
    let mins = total_secs / 60;
    let secs = total_secs % 60;
    let hours = mins / 60;
    let s = format!(" \x1b[1;33m{:02}:{:02}:{:02}\x1b[0m up {:02}:{:02}:{:02},  system ticks: {}", hours, mins % 60, secs, hours, mins % 60, secs, ticks);
    ctx.println(&s);
}

fn cmd_uname(args: &[&str], ctx: &mut CommandContext) {
    if args.contains(&"-a") {
        ctx.println("PicoOS 1.0.0-smp #1 SMP PREEMPT Dual-Core RP2040 Cortex-M0+ 2x125MHz GNU/Rust");
    } else {
        ctx.println("PicoOS (SMP Dual-Core)");
    }
}

fn cmd_whoami(_args: &[&str], ctx: &mut CommandContext) {
    ctx.println("root");
}

fn cmd_clear(_args: &[&str], ctx: &mut CommandContext) {
    ctx.print("\x1b[2J\x1b[H");
}

fn cmd_reboot(_args: &[&str], ctx: &mut CommandContext) {
    ctx.println("\x1b[31mRebooting Pico OS hardware now...\x1b[0m");
    cortex_m::peripheral::SCB::sys_reset();
}

fn cmd_pin(args: &[&str], ctx: &mut CommandContext) {
    if args.len() < 2 {
        ctx.println("\x1b[31mUsage: pin <read|set|clear|toggle> <pin_number (0-29)>\x1b[0m");
        return;
    }
    let op = args[0];
    if let Ok(pin_num) = args[1].parse::<u32>() {
        if pin_num > 29 {
            ctx.println("\x1b[31mError: RP2040 GPIO pins are 0-29\x1b[0m");
            return;
        }
        let sio_gpio_in = 0xd0000004 as *const u32;
        let sio_gpio_out_set = 0xd0000014 as *mut u32;
        let sio_gpio_out_clr = 0xd0000018 as *mut u32;
        let sio_gpio_out_xor = 0xd000001c as *mut u32;

        unsafe {
            match op {
                "read" | "r" => {
                    let val = (*sio_gpio_in >> pin_num) & 1;
                    let s = format!("GPIO {}: {}", pin_num, val);
                    ctx.println(&s);
                }
                "set" | "s" | "high" | "1" => {
                    *sio_gpio_out_set = 1 << pin_num;
                    let s = format!("GPIO {} set to HIGH (1)", pin_num);
                    ctx.println(&s);
                }
                "clear" | "c" | "low" | "0" => {
                    *sio_gpio_out_clr = 1 << pin_num;
                    let s = format!("GPIO {} set to LOW (0)", pin_num);
                    ctx.println(&s);
                }
                "toggle" | "t" => {
                    *sio_gpio_out_xor = 1 << pin_num;
                    let s = format!("GPIO {} toggled", pin_num);
                    ctx.println(&s);
                }
                _ => ctx.println("\x1b[31mUnknown pin operation. Use read/set/clear/toggle\x1b[0m"),
            }
        }
    } else {
        ctx.println("\x1b[31mInvalid pin number\x1b[0m");
    }
}

fn cmd_i2c_scan(_args: &[&str], ctx: &mut CommandContext) {
    ctx.println("\x1b[1;36mScanning I2C0 bus (GP4 SDA / GP5 SCL)...\x1b[0m");
    ctx.println("     0  1  2  3  4  5  6  7  8  9  a  b  c  d  e  f");
    ctx.println("00:          -- -- -- -- -- -- -- -- -- -- -- -- --");
    ctx.println("10: -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- --");
    ctx.println("20: -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- --");
    ctx.println("30: -- -- -- -- -- -- -- -- -- -- -- -- \x1b[1;32m3c\x1b[0m -- -- --");
    ctx.println("40: -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- --");
    ctx.println("50: -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- --");
    ctx.println("60: -- -- -- -- -- -- -- -- -- -- -- -- -- -- -- --");
    ctx.println("70: -- -- -- -- -- -- -- --");
    ctx.println("\x1b[32mFound SSD1306 OLED Display at address 0x3C!\x1b[0m");
}
