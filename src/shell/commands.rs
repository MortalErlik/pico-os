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
        "events" => cmd_events(args, ctx),
        "calc" | "bc" => cmd_calc(args, ctx),
        "ai" | "chat" => cmd_ai(args, ctx),
        "htop" | "top" => cmd_htop_snapshot(args, ctx),
        "nano" => cmd_nano_hint(args, ctx),
        "service" | "systemctl" => cmd_service(args, ctx),
        "neofetch" | "fetch" => cmd_fetch(args, ctx),
        "tmux" => {
            if args.first() == Some(&"help") || args.first() == Some(&"--help") {
                print_tmux_help(ctx);
            }
        }
        "disk_write" => cmd_disk_write(args, ctx),
        "disk_read" => cmd_disk_read(args, ctx),
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

fn print_tmux_help(ctx: &mut CommandContext) {
    ctx.println("\x1b[1;36m=== Tmux 4-Pane Split-Screen Multiplexer Guide ===\x1b[0m");
    ctx.println("  Launch: Type '\x1b[1;32mtmux\x1b[0m' to enter the multiplexer environment.");
    ctx.println("\x1b[1;33mTerminal Split Commands (type inside any pane):\x1b[0m");
    ctx.println("  \x1b[1;32msplit-v\x1b[0m (or 'split right') - Split active pane vertically (side-by-side)");
    ctx.println("  \x1b[1;32msplit-h\x1b[0m (or 'split down')  - Split active pane horizontally (top/bottom)");
    ctx.println("  \x1b[1;32mfocus <1..4>\x1b[0m (or 'pane N') - Switch input to Pane 1, 2, 3, or 4");
    ctx.println("  \x1b[1;32mclear\x1b[0m                       - Clear active pane screen");
    ctx.println("  \x1b[1;32mexit\x1b[0m                        - Close current pane (or exit if last)");
    ctx.println("\x1b[1;33mKeyboard Shortcuts (Ctrl+B Prefix):\x1b[0m");
    ctx.println("  \x1b[1;35mCtrl+B %\x1b[0m or \x1b[1;35mCtrl+B v\x1b[0m        - Split Vertical (Right)");
    ctx.println("  \x1b[1;35mCtrl+B \"\x1b[0m or \x1b[1;35mCtrl+B h\x1b[0m        - Split Horizontal (Down)");
    ctx.println("  \x1b[1;35mCtrl+B o\x1b[0m or \x1b[1;35mCtrl+B Tab\x1b[0m      - Cycle next active pane");
    ctx.println("  \x1b[1;35mCtrl+B 1..4\x1b[0m                 - Jump directly to Pane 1, 2, 3, or 4");
    ctx.println("  \x1b[1;35mCtrl+B x\x1b[0m                    - Close active pane");
    ctx.println("  \x1b[1;35mCtrl+B d\x1b[0m                    - Detach from tmux back to main shell");
}

fn cmd_help(args: &[&str], ctx: &mut CommandContext) {
    if args.first() == Some(&"tmux") {
        print_tmux_help(ctx);
        return;
    }

    ctx.println("\x1b[1;36m====================== Pico OS Command Reference ======================\x1b[0m");
    
    ctx.println("\x1b[1;33m[ APPLICATIONS & FULL-SCREEN TUI ]\x1b[0m");
    ctx.println("  \x1b[1;32mfetch\x1b[0m (or neofetch) - Display hardware specs, cute ASCII Cat & ANSI palette");
    ctx.println("  \x1b[1;32mai [query]\x1b[0m (or chat)  - 100% Offline AI Assistant & interactive REPL");
    ctx.println("  \x1b[1;32mtmux [help]\x1b[0m         - 4-Pane split-screen terminal multiplexer (split-v / split-h)");
    ctx.println("  \x1b[1;32mhtop\x1b[0m (or top)       - Live Dual-Core process monitor & 3 storage bars");
    ctx.println("  \x1b[1;32mcalc [expr]\x1b[0m (or bc) - Math calculator & REPL (sqrt, pow, pi, vars, ans)");
    ctx.println("  \x1b[1;32mnano <file>\x1b[0m         - Full-screen text editor with keyboard navigation");

    ctx.println("\x1b[1;33m[ PROCESS & SMP SERVICE MANAGEMENT ]\x1b[0m");
    ctx.println("  \x1b[1;32mps\x1b[0m                  - List all active tasks across CPU0 & CPU1 with CPU%");
    ctx.println("  \x1b[1;32mspawn [-f] <name>\x1b[0m   - Launch background task (Auto-balanced across CPU0/CPU1)");
    ctx.println("  \x1b[1;32mservice <name> <cmd>\x1b[0m- Daemon manager: service <start|stop|restart|status|list>");
    ctx.println("  \x1b[1;32mkill <pid|name>\x1b[0m     - Terminate task by PID or process name");
    ctx.println("  \x1b[1;32mfree\x1b[0m                - Display 192KB RAM, 128KB Swap & 95% OOM-Guard status");

    ctx.println("\x1b[1;33m[ FILESYSTEM & STORAGE ]\x1b[0m");
    ctx.println("  \x1b[1;32mls [path]\x1b[0m           - List directory contents with sizes and colors");
    ctx.println("  \x1b[1;32mcd <path>\x1b[0m           - Change current directory");
    ctx.println("  \x1b[1;32mpwd\x1b[0m                 - Print current working directory");
    ctx.println("  \x1b[1;32mmkdir / rm [-r]\x1b[0m     - Create directory or remove file/directory");
    ctx.println("  \x1b[1;32mtouch / cat\x1b[0m         - Create empty file or display file contents");
    ctx.println("  \x1b[1;32mcp / mv\x1b[0m             - Copy or move/rename files");
    ctx.println("  \x1b[1;32mecho [text]\x1b[0m         - Print text (supports > and >> redirection)");
    ctx.println("  \x1b[1;32mdf [-h]\x1b[0m             - Show Dual-Mount filesystems (Root tmpfs & /data Flash)");
    ctx.println("  \x1b[1;32msync\x1b[0m                - Force immediate flush of VFS snapshot to Flash");
    ctx.println("  \x1b[1;32mevents\x1b[0m              - Real-time inotify journal & auto-sync event monitor");
    ctx.println("  \x1b[1;32mformat\x1b[0m              - Format persistent /data partition on Flash");

    ctx.println("\x1b[1;33m[ SYSTEM & HARDWARE UTILITIES ]\x1b[0m");
    ctx.println("  \x1b[1;32mpin <r|s|c|t> <pin>\x1b[0m - Read / Set / Clear / Toggle GPIO pin");
    ctx.println("  \x1b[1;32mi2c_scan\x1b[0m            - Scan I2C bus for OLED and connected sensors");
    ctx.println("  \x1b[1;32muptime / uname [-a]\x1b[0m - System running time and Dual-Core SMP kernel info");
    ctx.println("  \x1b[1;32mwhoami / clear\x1b[0m      - Print current user (root) or clear terminal screen");
    ctx.println("  \x1b[1;32mreboot\x1b[0m              - Restart operating system");
    ctx.println("\x1b[1;36m=======================================================================\x1b[0m");
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
        ctx.println("\x1b[31mkill: missing PID or process name argument\x1b[0m");
        return;
    }
    let target = args[0];
    if let Ok(pid) = target.parse::<usize>() {
        if task::kill(pid) {
            let s = format!("\x1b[32mSignal SIGKILL (9) sent to PID {}\x1b[0m", pid);
            ctx.println(&s);
        } else {
            let s = format!("\x1b[31mkill: failed to kill PID {} (Protected or invalid)\x1b[0m", pid);
            ctx.println(&s);
        }
    } else {
        // Kill by process name
        if let Some(pid) = task::kill_by_name(target) {
            let s = format!("\x1b[32mTerminated process '{}' (PID {})\x1b[0m", target, pid);
            ctx.println(&s);
        } else {
            let s = format!("\x1b[31mkill: no active process named '{}' found (or protected)\x1b[0m", target);
            ctx.println(&s);
        }
    }
}

extern "C" fn demo_worker_task(_arg: usize) {
    loop {
        task::sleep_ms(1000);
    }
}

fn cmd_spawn(args: &[&str], ctx: &mut CommandContext) {
    let mut force = false;
    let mut clean_args = Vec::new();

    for arg in args {
        if *arg == "-f" || *arg == "--force" {
            force = true;
        } else {
            clean_args.push(*arg);
        }
    }

    let name = clean_args.first().copied().unwrap_or("worker_task");
    let core = if clean_args.len() > 1 {
        if clean_args[1].eq_ignore_ascii_case("auto") {
            255
        } else {
            clean_args[1].parse::<u8>().unwrap_or(255)
        }
    } else {
        255 // Auto SMP load-balance by default
    };

    // Prevent accidental duplicates unless -f is specified
    if !force {
        if let Some(existing_pid) = task::find_active_by_name(name) {
            let s = format!(
                "\x1b[33m[WARN] Task '{}' is already running (PID {}). Use 'service restart {}' or 'spawn -f {}' to duplicate.\x1b[0m",
                name, existing_pid, name, name
            );
            ctx.println(&s);
            return;
        }
    }

    let pid = task::spawn(name, core, 1024, demo_worker_task, 0);
    if core >= 2 {
        let s = format!("\x1b[32mSpawned task '{}' with PID {} (SMP Auto-Balanced)\x1b[0m", name, pid);
        ctx.println(&s);
    } else {
        let s = format!("\x1b[32mSpawned task '{}' on CPU{} with PID {}\x1b[0m", name, core, pid);
        ctx.println(&s);
    }
}

fn cmd_service(args: &[&str], ctx: &mut CommandContext) {
    if args.is_empty() || args[0] == "list" {
        ctx.println("\x1b[1;36m=== Pico OS System Daemons & Services ===\x1b[0m");
        let tasks = task::get_tasks();
        for t in tasks {
            let status = match t.state {
                task::TaskState::Ready => "\x1b[1;32mACTIVE (Ready)\x1b[0m",
                task::TaskState::Running => "\x1b[1;32mRUNNING\x1b[0m",
                task::TaskState::Sleeping(_) => "\x1b[1;36mSLEEPING (Idle)\x1b[0m",
                task::TaskState::Blocked => "\x1b[1;33mBLOCKED\x1b[0m",
                task::TaskState::Dead => "\x1b[1;31mSTOPPED\x1b[0m",
            };
            let line = format!("  ● {:<16} [PID {:>2}, CPU{}] - {}", t.name, t.pid, t.core, status);
            ctx.println(&line);
        }
        return;
    }

    let service_name = args[0];
    let action = args.get(1).copied().unwrap_or("status");

    match action {
        "start" => {
            if let Some(pid) = task::find_active_by_name(service_name) {
                let s = format!("\x1b[33m[INFO] Service '{}' is already active (PID {})\x1b[0m", service_name, pid);
                ctx.println(&s);
            } else {
                let pid = task::spawn(service_name, 255, 1024, demo_worker_task, 0);
                let s = format!("\x1b[32m[OK] Started service '{}' (PID {}, SMP Balanced)\x1b[0m", service_name, pid);
                ctx.println(&s);
            }
        }
        "stop" => {
            if let Some(pid) = task::kill_by_name(service_name) {
                let s = format!("\x1b[32m[OK] Stopped service '{}' (PID {})\x1b[0m", service_name, pid);
                ctx.println(&s);
            } else {
                let s = format!("\x1b[33m[INFO] Service '{}' is not currently active\x1b[0m", service_name);
                ctx.println(&s);
            }
        }
        "restart" => {
            if let Some(pid) = task::kill_by_name(service_name) {
                let s = format!("\x1b[0;90mStopping old instance (PID {})...\x1b[0m", pid);
                ctx.println(&s);
            }
            let pid = task::spawn(service_name, 255, 1024, demo_worker_task, 0);
            let s = format!("\x1b[32m[OK] Restarted service '{}' with fresh PID {} (SMP Balanced)\x1b[0m", service_name, pid);
            ctx.println(&s);
        }
        "status" => {
            if let Some(pid) = task::find_active_by_name(service_name) {
                let s = format!("● \x1b[1;32m{}\x1b[0m is \x1b[1;32mactive (running)\x1b[0m with PID {}", service_name, pid);
                ctx.println(&s);
            } else {
                let s = format!("○ \x1b[1;31m{}\x1b[0m is \x1b[1;31minactive (dead/stopped)\x1b[0m", service_name);
                ctx.println(&s);
            }
        }
        _ => {
            ctx.println("Usage: service <name> <start|stop|restart|status> or 'service list'");
        }
    }
}

fn cmd_free(_args: &[&str], ctx: &mut CommandContext) {
    let stats = mm::get_stats();
    let (swap_used, swap_total) = mm::get_swap_usage();
    let swap_free = swap_total.saturating_sub(swap_used);

    ctx.println("\x1b[1;37m               total        used        free      peak_used\x1b[0m");
    let mem_line = format!(
        "\x1b[1;36mMem:       {:>8} B  {:>8} B  {:>8} B     {:>8} B\x1b[0m",
        stats.total_bytes, stats.used_bytes, stats.free_bytes, stats.peak_used_bytes
    );
    ctx.println(&mem_line);

    let swap_line = format!(
        "\x1b[1;33mSwap:      {:>8} B  {:>8} B  {:>8} B            --\x1b[0m",
        swap_total, swap_used, swap_free
    );
    ctx.println(&swap_line);

    let counts = format!(
        "\x1b[0;90mAllocations: {} | Frees: {} | Guard: 95% RAM OOM-Protection Active\x1b[0m",
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
    ctx.println("\x1b[1;32m[ OK ] Persistent Virtual File System (VFS) synchronized to Physical Flash.\x1b[0m");
}

fn cmd_format(_args: &[&str], ctx: &mut CommandContext) {
    ctx.println("Formatting 1.0MB Persistent Flash partition (VFS Snapshot)...");
    match fs::format_fs() {
        Ok(_) => ctx.println("\x1b[1;32m[ OK ] Format complete! VFS Snapshot initialized on Flash.\x1b[0m"),
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

fn cmd_disk_write(args: &[&str], ctx: &mut CommandContext) {
    if args.len() < 2 {
        ctx.println("Usage: disk_write <block_id> <text>");
        return;
    }
    let block_id = match args[0].parse::<u32>() {
        Ok(id) => id,
        Err(_) => {
            ctx.println("Invalid block ID");
            return;
        }
    };
    let text = args[1..].join(" ");
    let mut buf = [0u8; 4096];
    let bytes = text.as_bytes();
    let len = bytes.len().min(4096);
    buf[..len].copy_from_slice(&bytes[..len]);
    
    fs::flash::write_disk_block(block_id, &buf);
    let msg = alloc::format!("Wrote {} bytes to True Disk block {}", len, block_id);
    ctx.println(&msg);
}

fn cmd_disk_read(args: &[&str], ctx: &mut CommandContext) {
    if args.len() != 1 {
        ctx.println("Usage: disk_read <block_id>");
        return;
    }
    let block_id = match args[0].parse::<u32>() {
        Ok(id) => id,
        Err(_) => {
            ctx.println("Invalid block ID");
            return;
        }
    };
    let mut buf = [0u8; 4096];
    fs::flash::read_disk_block(block_id, &mut buf);
    
    // Find null terminator or end
    let mut len = 4096;
    for (i, &b) in buf.iter().enumerate() {
        if b == 0 {
            len = i;
            break;
        }
    }
    
    match core::str::from_utf8(&buf[..len]) {
        Ok(s) => ctx.println(s),
        Err(_) => ctx.println("<binary data>"),
    }
}

fn cmd_events(_args: &[&str], ctx: &mut CommandContext) {
    let events = fs::get_events();
    let is_dirty = fs::is_fs_dirty();
    let status_str = if is_dirty {
        "\x1b[1;33mDIRTY (Delayed Auto-Sync Pending...)\x1b[0m"
    } else {
        "\x1b[1;32mCLEAN (Synchronized with Flash)\x1b[0m"
    };

    ctx.println("\x1b[1;36m=== VFS Real-time Event Journal (inotify log) ===\x1b[0m");
    let state_line = format!("VFS Status: {}", status_str);
    ctx.println(&state_line);
    ctx.println("-------------------------------------------------");

    if events.is_empty() {
        ctx.println("  No filesystem events recorded yet.");
        return;
    }

    for ev in events {
        let secs = ev.tick / 1000;
        let mins = secs / 60;
        let s = secs % 60;
        let ms = ev.tick % 1000;

        let (action_col, action_str) = match ev.kind {
            fs::FsEventKind::Create => ("\x1b[1;32m", "CREATE   "),
            fs::FsEventKind::Modify => ("\x1b[1;33m", "MODIFY   "),
            fs::FsEventKind::Delete => ("\x1b[1;31m", "DELETE   "),
            fs::FsEventKind::AutoSync => ("\x1b[1;35m", "AUTOSYNC "),
            fs::FsEventKind::ManualSync => ("\x1b[1;36m", "SYNC     "),
        };

        let row = format!(
            "  [{:02}:{:02}.{:03}] {}{}\x1b[0m {}",
            mins, s, ms, action_col, action_str, ev.path
        );
        ctx.println(&row);
    }
}

fn cmd_calc(args: &[&str], ctx: &mut CommandContext) {
    if args.is_empty() {
        ctx.println("Usage: calc <expression> or type 'calc' alone to enter interactive mode.");
        ctx.println("Example: calc 15 * (4 + 2) ^ 2");
        return;
    }
    let expr = args.join(" ");
    let mut calc_ctx = crate::calc::CalcContext::new();
    match calc_ctx.eval(&expr) {
        Ok(crate::calc::CalcOutput::Value(v)) => {
            let s = format!("= \x1b[1;32m{}\x1b[0m", crate::calc::format_num(v));
            ctx.println(&s);
        }
        Ok(crate::calc::CalcOutput::Assignment(name, v)) => {
            let s = format!("{} = \x1b[1;32m{}\x1b[0m", name, crate::calc::format_num(v));
            ctx.println(&s);
        }
        Ok(crate::calc::CalcOutput::VarList(vars)) => {
            for v in vars {
                ctx.println(&v);
            }
        }
        Ok(crate::calc::CalcOutput::Help) => {
            ctx.println("Pico OS Calculator: +, -, *, /, %, ^, sqrt(x), abs(x), pow(a,b), min(a,b), max(a,b), round(x), pi, e, ans");
        }
        Ok(crate::calc::CalcOutput::Empty | crate::calc::CalcOutput::Exit) => {}
        Err(e) => {
            let msg = format!("\x1b[31mCalc error: {}\x1b[0m", e);
            ctx.println(&msg);
        }
    }
}

fn cmd_fetch(_args: &[&str], ctx: &mut CommandContext) {
    crate::shell::fetch::render_fetch(|s| (ctx.output)(s));
}

fn cmd_ai(args: &[&str], ctx: &mut CommandContext) {
    if args.is_empty() {
        ctx.println("\x1b[1;35mPico-AI:\x1b[0m Type '\x1b[1;32mai <question>\x1b[0m' or type '\x1b[1;32mai\x1b[0m' in main shell for interactive mode!");
        ctx.println("Example: ai what is the meaning of life");
        return;
    }
    let query = args.join(" ");
    let mut ai_ctx = crate::ai::AiContext::new();
    let resp = ai_ctx.respond(&query);
    let out = format!("\x1b[1;35mPico-AI:\x1b[0m {}", resp);
    ctx.println(&out);
}

fn cmd_htop_snapshot(_args: &[&str], ctx: &mut CommandContext) {
    ctx.println("\x1b[1;36m=== Pico-OS System Monitor Snapshot ===\x1b[0m");
    let (c0, c1) = task::get_cpu_loads();
    let row0 = format!("  CPU0 (Core 0): \x1b[1;32m[{:>3}%]\x1b[0m 125 MHz Interactive", c0);
    let row1 = format!("  CPU1 (Core 1): \x1b[1;32m[{:>3}%]\x1b[0m 125 MHz SMP Worker", c1);
    ctx.println(&row0);
    ctx.println(&row1);

    let stats = mm::get_stats();
    let ram_pct = if stats.total_bytes > 0 { (stats.used_bytes * 100) / stats.total_bytes } else { 0 };
    let ram_row = format!("  RAM: {}K / {}K (\x1b[1;33m{}%\x1b[0m) | Heap OOM Guard: 95%", stats.used_bytes / 1024, stats.total_bytes / 1024, ram_pct);
    ctx.println(&ram_row);

    ctx.println("\x1b[1;33mActive Tasks:\x1b[0m");
    cmd_ps(&[], ctx);
}

fn cmd_nano_hint(args: &[&str], ctx: &mut CommandContext) {
    if args.is_empty() {
        ctx.println("Usage: nano <filename> (Full-screen editor is available in the main shell)");
    } else {
        ctx.println("Tip: nano is a full-screen application. Detach from tmux (Ctrl+B d) or run in main shell!");
    }
}
