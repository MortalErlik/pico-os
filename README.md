# 🍓 Pico-OS: Bare-Metal Dual-Core SMP Operating System in Rust & ARM Assembly

[![Language: Rust](https://img.shields.io/badge/Language-Rust%20%28no__std%29-orange.svg)](https://www.rust-lang.org/)
[![Target: RP2040](https://img.shields.io/badge/Target-RP2040%20Cortex--M0%2B-blue.svg)](https://www.raspberrypi.com/products/rp2040/)
[![Architecture: Dual-Core SMP](https://img.shields.io/badge/Arch-Dual--Core%20SMP-green.svg)]()
[![License: MIT](https://img.shields.io/badge/License-MIT-purple.svg)]()

**Pico-OS** is a lightweight, bare-metal Unix-like operating system designed and implemented from scratch in **Rust and ARM Assembly** for the **Raspberry Pi Pico (RP2040 Dual-Core Cortex-M0+)**. 

It features True Symmetric Multiprocessing (SMP), dynamic load balancing across CPU0/CPU1, a custom heap allocator with 95% OOM-Killer protection, a dual-mount Virtual File System (VFS) with delayed physical flash committing, and full-featured interactive TUI applications (`tmux`, `htop`, `nano`, `calc`, `fetch`).

---

## 📸 Interactive Terminal Showcase

### 🐱 1. `fetch` / `neofetch` (Hardware Specs & Cute ASCII Kitty)
```text
root@pico:/# fetch
   /\_/\        root@pico
  ( o.o )       -----------------------------------
   > ^ <        OS:      Pico-OS Dual-Core SMP (v0.1.0)
  /  ~  \       Host:    Raspberry Pi Pico (RP2040 Cortex-M0+)
 /|     |\      Kernel:  6.1.0-picos-smp (125 MHz Dual-Core)
(_|     |_)     Uptime:  00:04:12
  (_____)       Tasks:   5 total (CPU0: 3, CPU1: 2)
                Memory:  6K / 192K (Swap: 0K / 128K)
                Storage: VFS 256K | Raw Disk 1.4MB | Swap 128K
                Shell:   picos-sh v1.0
                Guard:   95% RAM OOM-Protection Active

                 ███ ███ ███ ███ ███ ███ ███ ███
                 ███ ███ ███ ███ ███ ███ ███ ███
```

### 📊 2. `htop` (Dual-Core SMP Process & Storage Monitor)
```text
0[||||||||||||||||||||  48%]   Tasks: 4 total, 2 running
1[||||||||||||          24%]   Uptime: 00:12:45
Mem[||||||              6K/192K]   Arch: Dual-Core SMP (RP2040)
VFS [|                   1K/256K]   Disk: VFS Snapshot
Swap[                    0K/128K]   Disk: Application Paging
Raw [                    0M/1.4M]   Disk: True Block Device

 PID CORE USER  STATE  CPU%  STACK  NAME
   1 CPU0 root  RUN     28%    16B  kernel_core0
   2 CPU1 root  RUN     24%    36B  rt_worker_core1
   3 CPU0 root  READY    0%    12B  vfs_daemon
   4 CPU0 root  RUN     20%    12B  worker_task

 F1 Help  F9/K Kill Process  F10/Q Quit Htop
```

### 🖥️ 3. `tmux` (4-Pane Split-Screen Terminal Multiplexer)
```text
┌── [* P1] ─────────────────────────┐┌── [  P2] ─────────────────────────┐
│ $ fetch                           ││ $ calc                            │
│    /\_/\       OS: Pico-OS SMP    ││ calc> sqrt(144) * pi              │
│   ( o.o )      Uptime: 00:12:45   ││ = 37.69911                        │
│    > ^ <       Memory: 6K / 192K  ││ calc> ans + 10                    │
│                                   ││ = 47.69911                        │
│ >                                 ││ >                                 │
└───────────────────────────────────┘└─── ───────────────────────────────┘
┌── [  P3] ─────────────────────────┐┌── [  P4] ─────────────────────────┐
│ $ service list                    ││ $ df -h                           │
│   ● kernel_core0    [RUNNING]     ││ Filesystem  Size  Used  Avail  %  │
│   ● rt_worker_core1 [RUNNING]     ││ / (tmpfs)   256K    1K   255K  1% │
│   ● vfs_daemon      [READY]       ││ /data (fls) 1.0M    4K  1020K  1% │
│                                   ││                                   │
│ >                                 ││ >                                 │
└───────────────────────────────────┘└─── ───────────────────────────────┘
[pico-tmux] Panes: 4/4 (Active: Pane 1) |  "RP2040 SMP" 12:45
```

---

## 💾 Deep Dive: 2.0 MB Physical Flash Memory Map & Storage Engine

Pico-OS implements a complete multi-tier embedded storage architecture that organizes the Raspberry Pi Pico's **2.0 MB (2048 KB) W25Q080 SPI NOR Flash** with byte-precise partitioning:

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                 2.0 MB (2048 KB) PHYSICAL FLASH MEMORY MAP                  │
├──────────────┬──────────────┬──────────┬────────────────────────────────────┤
│ Memory Range │ Flash Offset │   Size   │ Description & Purpose              │
├──────────────┼──────────────┼──────────┼────────────────────────────────────┤
│ 0x1000_0000  │ 0x000000     │ 256 B    │ rp2040-boot2 (Quad-SPI Bootloader) │
│ 0x1000_0100  │ 0x000100     │ ~70 KB   │ Pico-OS Kernel Binary & RoData     │
│ 0x1001_2000  │ 0x012000     │ 440 KB   │ Reserved Kernel Growth Space       │
│ 0x1008_0000  │ 0x080000     │ 128 KB   │ Virtual Memory Swap Paging Space   │
│ 0x100A_0000  │ 0x0A0000     │ 384 KB   │ Raw Block Device & Inotify Logs    │
│ 0x1010_0000  │ 0x100000     │ 1024 KB  │ /data Persistent Partition (1.0MB) │
└──────────────┴──────────────┴──────────┴────────────────────────────────────┘
```

> ### 💡 Why is the compiled binary/UF2 only ~70 KB? Where does the rest of the 2 MB go?
> Pico-OS is written in zero-overhead, bare-metal `no_std` Rust and handcrafted ARM Assembly. The entire operating system kernel (multiprocessing scheduler, custom heap allocator, TUI applications, calculator, VFS, and drivers) compiles to just **~70 KB of ultra-optimized native machine code**.
> 
> Instead of leaving the remaining **1.93 MB** of high-speed onboard SPI NOR flash empty, Pico-OS partitions and utilizes the physical chip completely:
> 1. **1024 KB (1.0 MB)** is dedicated to the persistent physical file system (`/data`).
> 2. **128 KB** is dedicated to application virtual memory swap paging.
> 3. **384 KB** is dedicated to raw block storage and journaled inotify logs.
> 4. **Zero Bloat, 100% Efficiency!**

---

### Storage Stack Architecture

```text
┌─────────────────────────────────────────────────────────────────────────┐
│                           PICO-OS STORAGE STACK                         │
├─────────────────────────────────────────────────────────────────────────┤
│  User Space / CLI:   ls, cat, touch, echo >, nano, df -h, sync, events  │
├─────────────────────────────────────────────────────────────────────────┤
│  Virtual File System (VFS): Hierarchical Inode Tree & Directory Table   │
├───────────────────────────────────┬─────────────────────────────────────┤
│      / (Root tmpfs, 256 KB)       │     /data (Persistent Flash, 1.0MB) │
│  - Instant In-Memory Operations   │  - Sector 0x100000..0x200000 (NOR)  │
│  - Zero Flash Wear for temp files │  - Survives power loss & reboot     │
├───────────────────────────────────┴─────────────────────────────────────┤
│  Auto-Sync Journal & Wear-Leveling Daemon (vfs_daemon):                 │
│  - Tracks dirty inodes & file modification timestamps                   │
│  - 2.5-Second Idle Grace Period: coalesces burst writes                 │
│  - Atomic Magic Header Snapshot Commit (`PICO_VFS_SNAPSHOT_V1`)         │
├─────────────────────────────────────────────────────────────────────────┤
│  Physical Media & Swap Layer:                                           │
│  - 1.4 MB Raw Block Device Space                                        │
│  - 128 KB Paging Swap Space (32 x 4KB pages bitmap)                     │
│  - Flash XIP & Core 1 SMP Bus Arbitration Safety (`CORE1_SPAWNED`)      │
└─────────────────────────────────────────────────────────────────────────┘
```

#### 1. Dual-Partition Virtual File System
* **`/` (Root tmpfs Partition - 256 KB)**: Fast, non-blocking RAM directory structure (`/bin`, `/etc`, `/dev`, `/proc`, `/home`). Ideal for volatile files and fast CLI pipes without wearing flash memory.
* **`/data` (Persistent Physical Flash Partition - 1.0 MB)**: Located at offset `0x100000` (last 1MB of onboard 2MB SPI NOR Flash). All files written to `/data` survive power loss and reboots.

#### 2. Smart Auto-Commit & Wear-Leveling Journal
* Microcontroller NOR Flash has a finite number of write/erase cycles (typically ~100,000 per 4KB sector). Writing directly to flash on every keystroke would degrade flash memory rapidly.
* **Delayed Auto-Sync**: When a file is created or updated:
  1. The file change is marked **dirty** in RAM immediately.
  2. An event is published to the `events` inotify journal.
  3. The `vfs_daemon` task arms a **2.5-second timer**.
  4. If additional writes occur within 2.5s, the timer resets (write coalescing).
  5. Once the system is idle for 2.5s, `vfs_daemon` automatically flushes the snapshot to physical flash with a single atomic transaction.
* **Manual Immediate Sync**: Users can run `sync` at any time to bypass the timer and force an immediate flash commit.

#### 3. Application Swap Paging Area
* A dedicated **128 KB Swap Partition** (divided into 32x 4KB pages) is managed via an atomic bitmask (`SWAP_BITMAP`).
* Memory usage and swap consumption can be monitored in real-time using `free` and the `Swap[` meter in `htop`.

#### 4. SMP-Safe Physical Flash Bus Arbitration
* On the RP2040, the physical SPI NOR Flash is accessed via the XIP (Execute-In-Place) bus. Writing to or erasing flash sectors requires entering raw serial mode, which can cause CPU hangs if another core attempts to fetch code.
* Pico-OS coordinates flash writes between Core 0 and Core 1 using `critical_section` locks and `CORE1_SPAWNED` atomic barriers to guarantee 100% bus collision-free operations.

---

## 🌟 Key Features

### ⚡ 1. Dual-Core SMP Architecture & Task Scheduling
* **True Symmetric Multiprocessing**: Core 0 runs the interactive shell and kernel daemons, while Core 1 operates as a real-time worker core.
* **ARM Cortex-M0+ Assembly Context Switcher**: Custom Thumb-1 low (`r4-r7`) and high (`r8-r11`) register save/restore using `PendSV` interrupt.
* **Dynamic SMP Load Balancing**: `Scheduler::spawn` automatically routes new tasks to the least loaded CPU core, with active task density tie-breaking.
* **Preemption**: 1000 Hz `SysTick` timer interrupt for millisecond-precision round-robin scheduling.

### 🧠 2. Memory Management & 95% OOM Guard
* **First-Fit Linked-List Heap Allocator**: 192 KB total heap with automatic contiguous block coalescing upon deallocation.
* **95% RAM OOM-Killer**: Automatically detects critical memory pressure (>95%) and terminates non-essential user tasks (PID > 3) while protecting kernel daemons.
* **128 KB Swap Partition Tracker**: Live bitmap page tracker exposed through `free` and `htop`.

### 🖥️ 3. Interactive Terminal Applications (TUI)
* **`tmux` 4-Pane Split-Screen Multiplexer**:
  * 1 to 4 split panes (Single, Horizontal, Vertical, Triple, and 2x2 Grid).
  * Built-in split commands: `split-v` / `split right`, `split-h` / `split down`, `focus 1..4`.
  * Keyboard shortcuts: `Ctrl+B %` / `v` (vertical), `Ctrl+B "` / `h` (horizontal), `Ctrl+B o` (cycle pane), `Ctrl+B 1..4`, `Ctrl+B x` (close), `Ctrl+B d` (detach).
  * Live green status bar pinned at the bottom row.
* **`htop` Live Process Monitor**:
  * Real-time dual-core CPU% bars, RAM (Used/Total), VFS, Swap, and Raw Disk meters.
  * Task table with interactive PID kill (`F9` / `k` / `K`).
* **`calc` (or `bc`) Math Engine**:
  * Supports `+`, `-`, `*`, `/`, `%`, `^`, `x` / `X` multiplication (`50x4 = 200`), implicit multiplication (`2(3+4)`).
  * Functions: `sqrt`, `abs`, `pow`, `min`, `max`, `round`, `pi`, `e`, variable assignment (`x = 10`), and `ans`.
* **`nano <file>` Text Editor**:
  * Full-screen ANSI editor with scrolling, arrow-key navigation, `Ctrl+O` save, and `Ctrl+X` exit.
* **`fetch` (or `neofetch`)**:
  * System hardware specifications, uptime, tasks, memory/swap, cute colored ASCII Kitty logo, and 16-color ANSI test palette.

### ⚙️ 4. Linux Service Manager (`service` / `systemctl`)
* **Singleton Daemon Protection**: Prevents duplicate task instances unless forced with `-f`.
* **Service Control**: `service <name> <start|stop|restart|status>` and `service list`.

### 🔌 5. Hardware & Peripheral Control
* **Dual Terminal I/O**: Interactive shell is active over USB CDC-ACM Serial and Hardware UART0.
* **GPIO & Bus Control**: `pin` command for GPIO read/set/clear/toggle and `i2c_scan` for bus probing.
* **ESP-01 Power & Pin Configuration**: Hardware setup for `GP2 RST`, `GP3 IO0`, and `GP6 CH_PD`.

---

## 🗺️ Planned Hardware Features (Roadmap)

The following peripheral integrations are in active development:

- [ ] **ESP8266 (ESP-01) Wi-Fi Modem Driver (UART0)**:
  - Non-blocking AT command parser & Wi-Fi station manager (`wifi connect`, `wifi scan`).
  - Lightweight TCP/IP socket stack and HTTP client (`curl`, `ping`, network time NTP).
- [ ] **SSD1306 0.96" OLED Display Driver (I2C0)**:
  - 128x64 monochrome frame buffer rendering via `embedded-graphics`.
  - Real-time hardware system dashboard (CPU load, RAM usage bar, Uptime, Wi-Fi status).

---

## 📌 Hardware Pinout Map (RP2040 + ESP-01 + SSD1306)

| Physical Pin | Pico Pin | Connection | Function | Status |
| :--- | :--- | :--- | :--- | :--- |
| **Pin 1** | GP0 | ESP-01 RXD / Serial TX | Pico → ESP Serial TX | Configured |
| **Pin 2** | GP1 | ESP-01 TXD / Serial RX | ESP → Pico Serial RX | Configured |
| **Pin 4** | GP2 | ESP-01 RST | Hardware Reset Control | Configured |
| **Pin 5** | GP3 | ESP-01 IO0 | Boot / Flash Mode Control | Configured |
| **Pin 6** | GP4 | OLED SDA | I2C0 Data Line | Configured |
| **Pin 7** | GP5 | OLED SCL | I2C0 Clock Line | Configured |
| **Pin 9** | GP6 | ESP-01 CH_PD | Chip Enable / Power Control | Configured |
| **Pin 36** | 3V3 (OUT) | OLED + ESP-01 VCC | 3.3V Power Output | Power |
| **Pin 38** | GND | Common GND | Ground | Power |
| **Pin 39** | VSYS | Power Input | 5V Power / Switch | Power |

---

## 🚀 Getting Started & Flashing

### Prerequisites
* Rust toolchain with target `thumbv6m-none-eabi`:
  ```bash
  rustup target add thumbv6m-none-eabi
  cargo install elf2uf2-rs --locked
  ```

### Build & Generate UF2
```bash
cargo build --release
elf2uf2-rs target/thumbv6m-none-eabi/release/pico_os pico_os.uf2
```

### Flash to Raspberry Pi Pico
1. Hold down the **BOOTSEL** button on the Raspberry Pi Pico while connecting USB.
2. Mounts as `RPI-RP2` drive.
3. Copy `pico_os.uf2`:
   ```bash
   cp pico_os.uf2 /run/media/$USER/RPI-RP2/
   ```
4. Pico reboots immediately into Pico-OS!

---

## 💻 Connecting via Serial Terminal

Connect via any serial terminal at **115200 baud**:
```bash
picocom -b 115200 /dev/ttyACM0
# or
screen /dev/ttyACM0 115200
```

```text
  ____  _            ____   ____  
 |  _ \(_) ___ ___  / __ \ / ___| 
 | |_) | |/ __/ _ \| |  | |\___ \ 
 |  __/| | (_| (_) | |__| | ___) |
 |_|   |_|\___\___/ \____/ |____/ 
 Custom Bare-Metal OS in Rust & Assembly on RP2040 Dual-Core SMP
 Developed for Raspberry Pi Pico + ESP8266 (ESP-01) + SSD1306 OLED
 Apps & Tools: fetch | tmux | htop | calc | nano | service list
 Type 'help' for command reference or 'tmux help' for split-screen guide.

root@pico:/# fetch
```

---

## 📜 License
Licensed under the MIT License.
