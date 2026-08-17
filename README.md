# Pico-OS

[![Language: Rust](https://img.shields.io/badge/Language-Rust%20%28no__std%29-orange.svg)](https://www.rust-lang.org/)
[![Target: RP2040](https://img.shields.io/badge/Target-RP2040%20Cortex--M0%2B-blue.svg)](https://www.raspberrypi.com/products/rp2040/)
[![Architecture: Dual-Core SMP](https://img.shields.io/badge/Arch-Dual--Core%20SMP-green.svg)]()
[![License: MIT](https://img.shields.io/badge/License-MIT-purple.svg)]()

**Pico-OS** is a lightweight, bare-metal Unix-like operating system designed and implemented from scratch in **Rust and ARM Assembly** for the **Raspberry Pi Pico (RP2040 Dual-Core Cortex-M0+)**.

The goal of this project is to push the limits of a $4 microcontroller (264 KB SRAM, 2 MB SPI Flash) by implementing True Symmetric Multiprocessing (SMP), dynamic load balancing, a custom heap allocator with an OOM-Killer, a dual-mount Virtual File System (VFS), and full-featured interactive TUI applications (`tmux`, `htop`, `nano`, `calc`, `fetch`)—all packed into a ~350 KB binary with zero dependencies.

---

## Interactive Terminal Showcase

### 1. `fetch` / `neofetch` (Hardware Specs)
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
```

### 2. `htop` (Dual-Core SMP Process & Storage Monitor)
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

### 3. `tmux` (4-Pane Split-Screen Terminal Multiplexer)
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

### 4. `ai` / `chat` (Offline Rule-Based Assistant / Easter Egg)
A zero-allocation, flash-resident rule-based chatbot implemented as a fun easter egg for terminal entertainment. It generates basic code snippets, technical trivia, and witty developer banter using a static dataset stored entirely in NOR Flash RoData—consuming 0 bytes of dynamic heap.

```text
root@pico:/# ai
=================================================================
  PICO-AI: RULE-BASED CONVERSATIONAL ENGINE (v0.4)
  100% Offline Local | Zero-Allocation | RP2040 Dual-Core SMP
=================================================================
Type your thoughts or 'exit' to return to shell.

you> write python calculator
Pico-AI: Here is a complete, single-file Interactive Python Calculator REPL:
```python
import math

def run_calculator():
    print('=== Pico Python REPL Calculator ===')
    while True:
        line = input('calc> ').strip()
        if line in ('exit', 'quit'): break
        print('->', eval(line, {'__builtins__': None}, {'sqrt': math.sqrt, 'pi': math.pi}))

run_calculator()
```

---

## 2.0 MB Physical Flash Memory Map & Storage Engine

Pico-OS implements a complete multi-tier embedded storage architecture that organizes the Raspberry Pi Pico's **2.0 MB (2048 KB) W25Q080 SPI NOR Flash** with byte-precise partitioning:

```text
┌─────────────────────────────────────────────────────────────────────────────┐
│                 2.0 MB (2048 KB) PHYSICAL FLASH MEMORY MAP                  │
├──────────────┬──────────────┬──────────┬────────────────────────────────────┤
│ Memory Range │ Flash Offset │   Size   │ Description & Purpose              │
├──────────────┼──────────────┼──────────┼────────────────────────────────────┤
│ 0x1000_0000  │ 0x000000     │ 256 B    │ rp2040-boot2 (Quad-SPI Bootloader) │
│ 0x1000_0100  │ 0x000100     │ ~350 KB  │ Pico-OS Kernel & App RoData        │
│ 0x1005_8000  │ 0x058000     │ 160 KB   │ Reserved Kernel Growth Space       │
│ 0x1008_0000  │ 0x080000     │ 128 KB   │ Virtual Memory Swap Paging Space   │
│ 0x100A_0000  │ 0x0A0000     │ 384 KB   │ Raw Block Device & Inotify Logs    │
│ 0x1010_0000  │ 0x100000     │ 1024 KB  │ /data Persistent Partition (1.0MB) │
└──────────────┴──────────────┴──────────┴────────────────────────────────────┘
```

The entire operating system kernel (scheduler, allocator, TUI apps, VFS, and drivers) compiles to just **~350 KB**. The remaining flash is utilized for the persistent filesystem (`/data`), application virtual memory swap, and raw block storage.

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
* **`/` (Root tmpfs Partition - 256 KB)**: Fast, non-blocking RAM directory structure. Ideal for volatile files and fast CLI pipes without wearing flash memory.
* **`/data` (Persistent Physical Flash Partition - 1.0 MB)**: Located at offset `0x100000`. All files written to `/data` survive power loss and reboots.

#### 2. Smart Auto-Commit & Wear-Leveling Journal
* Microcontroller NOR Flash has a finite number of write/erase cycles. 
* **Delayed Auto-Sync**: When a file is updated, it is marked dirty in RAM and logged. The `vfs_daemon` arms a 2.5-second timer. If additional writes occur, the timer resets (write coalescing). Once idle, it flushes the snapshot to physical flash atomically.
* Users can run `sync` to bypass the timer and force an immediate flash commit.

#### 3. Application Swap Paging Area
* A dedicated **128 KB Swap Partition** (divided into 32x 4KB pages) is managed via an atomic bitmask (`SWAP_BITMAP`).

#### 4. SMP-Safe Physical Flash Bus Arbitration
* Erasing flash sectors requires entering raw serial mode, which can cause CPU hangs if another core attempts to fetch code over the XIP bus.
* Pico-OS coordinates flash writes between Core 0 and Core 1 using `critical_section` locks and atomic barriers to guarantee 100% bus collision-free operations.

---

## Key Features

### 1. Dual-Core SMP Architecture & Task Scheduling
* **True Symmetric Multiprocessing**: Core 0 runs the interactive shell and kernel daemons; Core 1 operates as a real-time worker core.
* **ARM Cortex-M0+ Context Switcher**: Custom Thumb-1 low and high register save/restore using `PendSV`.
* **Dynamic SMP Load Balancing**: `Scheduler::spawn` automatically routes new tasks to the least loaded CPU core.
* **Preemption**: 1000 Hz `SysTick` timer interrupt for millisecond-precision round-robin scheduling.

### 2. Memory Management & OOM Guard
* **First-Fit Linked-List Heap Allocator**: 192 KB total heap with automatic contiguous block coalescing.
* **95% RAM OOM-Killer**: Automatically detects critical memory pressure (>95%) and terminates non-essential user tasks while protecting kernel daemons.

### 3. Interactive Terminal Applications (TUI)
* **`tmux` 4-Pane Split-Screen Multiplexer**: Support for vertical/horizontal splits (`Ctrl+B %` / `Ctrl+B "`) and cycling panes.
* **`htop` Live Process Monitor**: Real-time dual-core CPU% bars, RAM, Swap, and interactive task termination (`F9`/`K`).
* **`calc` (or `bc`) Math Engine**: Evaluates math expressions with support for implicit multiplication and functions (`sqrt`, `pi`, variable assignment).
* **`nano <file>` Text Editor**: ANSI editor with scrolling, `Ctrl+O` save, and `Ctrl+X` exit.

### 4. Linux Service Manager (`service` / `systemctl`)
* **Singleton Daemon Protection**: Prevents duplicate task instances.
* **Service Control**: `service <name> <start|stop|restart|status>` and `service list`.

### 5. Hardware & Peripheral Control
* **Dual Terminal I/O**: Interactive shell is active over USB CDC-ACM Serial and Hardware UART0.
* **GPIO & Bus Control**: `pin` command for GPIO read/set/clear/toggle and `i2c_scan` for bus probing.

---

## Roadmap

The following peripheral integrations are in active development:

- [ ] **ESP8266 (ESP-01) Wi-Fi Modem Driver (UART0)**:
  - Non-blocking AT command parser & Wi-Fi station manager.
  - Lightweight TCP/IP socket stack and HTTP client (`curl`, `ping`).
- [ ] **SSD1306 0.96" OLED Display Driver (I2C0)**:
  - 128x64 monochrome frame buffer rendering via `embedded-graphics`.
  - Real-time hardware system dashboard.

---

## Hardware Pinout Map (RP2040 + ESP-01 + SSD1306)

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

## Quick Start Guide: Installation & Flashing

### Step 1: Install Build Toolchain (Optional if using prebuilt UF2)
```bash
# Install Rust embedded target for Cortex-M0+
rustup target add thumbv6m-none-eabi

# Install UF2 conversion tool
cargo install elf2uf2-rs --locked
```

### Step 2: Build the Firmware
```bash
git clone https://github.com/MortalErlik/pico-os.git
cd pico-os
cargo build --release
elf2uf2-rs target/thumbv6m-none-eabi/release/pico_os pico_os.uf2
```

### Step 3: Flash to Raspberry Pi Pico (Drag & Drop)
1. **Hold down the white BOOTSEL button** on your Raspberry Pi Pico while plugging the Micro-USB cable into your computer.
2. The Pico will mount as a mass-storage drive named **`RPI-RP2`**.
3. Copy the `pico_os.uf2` file into the `RPI-RP2` drive.
4. The Raspberry Pi Pico will automatically unmount, reboot, and boot straight into **Pico-OS**!

---

## Connecting to the Interactive Shell

Pico-OS exposes a full VT100/ANSI interactive Unix terminal over both **USB CDC-ACM (Micro-USB port)** and **Hardware UART0 (GP0/GP1)** at **115200 Baud (8N1)**.

### Linux (Recommended)
Add your user to the dialout group if needed (`sudo usermod -aG dialout,uucp $USER`):
```bash
# Using picocom (recommended):
picocom -b 115200 /dev/ttyACM0

# Or using minicom:
minicom -b 115200 -D /dev/ttyACM0

# Or using screen:
screen /dev/ttyACM0 115200
```

### macOS
```bash
screen /dev/tty.usbmodem* 115200
```

### Windows
1. Open **Device Manager** to find your Pico's COM port (e.g. `COM3`).
2. Open **PuTTY**:
   - Connection type: **Serial**
   - Serial line: `COM3`
   - Speed: `115200`
   - Click **Open**.

---

## First Things to Try in Pico-OS

Once connected, press `Enter` to see the prompt. Try running these commands right away:

1. **`fetch`**: Display the system hardware specs and ASCII Kitty logo!
2. **`htop`**: Launch the live dual-core monitor (`q` to exit, `k` to kill tasks).
3. **`tmux`**: Open the 4-pane terminal multiplexer (`split-v`, `split-h`, `focus 1..4`, `Ctrl+B d` to detach).
4. **`ai` / `chat`**: Chat with Pi-Copilot or generate code (`ai write python calculator`).
5. **`calc 50x4 + sqrt(144)`**: Evaluate math expressions directly.
6. **`nano test.txt`**: Create and edit files (`Ctrl+O` save, `Ctrl+X` exit).
7. **`service list`**: Inspect background daemons across CPU0 and CPU1.
8. **`help`**: View the full command manual.

---

## License
Licensed under the MIT License.
