# 🍓 Pico-OS: Bare-Metal Dual-Core SMP Operating System in Rust & ARM Assembly

[![Language: Rust](https://img.shields.io/badge/Language-Rust%20%28no__std%29-orange.svg)](https://www.rust-lang.org/)
[![Target: RP2040](https://img.shields.io/badge/Target-RP2040%20Cortex--M0%2B-blue.svg)](https://www.raspberrypi.com/products/rp2040/)
[![Architecture: Dual-Core SMP](https://img.shields.io/badge/Arch-Dual--Core%20SMP-green.svg)]()
[![License: MIT](https://img.shields.io/badge/License-MIT-purple.svg)]()

**Pico-OS** is a lightweight, bare-metal Unix-like operating system designed and implemented from scratch in **Rust and ARM Assembly** for the **Raspberry Pi Pico (RP2040 Dual-Core Cortex-M0+)**. 

It features True Symmetric Multiprocessing (SMP), dynamic load balancing across CPU0/CPU1, a custom heap allocator with 95% OOM-Killer protection, an in-memory Virtual File System (VFS) with delayed physical flash committing, interactive TUI applications (`tmux`, `htop`, `nano`, `calc`, `fetch`), and hardware drivers for OLED (I2C0) and ESP8266 (UART0).

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

### 💾 3. Dual-Mount Filesystem & Auto-Sync Journal
* **Dual Partition Architecture**:
  * `/` : Fast in-memory root filesystem (`tmpfs`).
  * `/data` : 1.0 MB Persistent Physical Flash partition.
* **Auto-Sync Journal**: Delayed auto-commit daemon syncs modified files to physical flash after 2.5s idle to minimize flash write wear.
* **File Operations**: `ls`, `cd`, `pwd`, `mkdir`, `rm -r`, `touch`, `cat`, `cp`, `mv`, `echo` (with `>` and `>>` redirection), `df -h`, `sync`, `events`, `format`.

### 🖥️ 4. Interactive Terminal Applications (TUI)
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

### ⚙️ 5. Linux Service Manager (`service` / `systemctl`)
* **Singleton Daemon Protection**: Prevents duplicate task instances unless forced with `-f`.
* **Service Control**: `service <name> <start|stop|restart|status>` and `service list`.

### 🔌 6. Hardware Peripherals
* **Dual Terminal I/O**: Shell simultaneously active over USB CDC-ACM Serial and Hardware UART0.
* **SSD1306 OLED Display (I2C0 GP4/GP5)**: Real-time graphical dashboard showing CPU%, RAM bar, and uptime.
* **ESP-01 (ESP8266) Management**: Reset, boot, and power control (`GP2 RST`, `GP3 IO0`, `GP6 CH_PD`).

---

## 📌 Hardware Pinout Map

| Physical Pin | Pico Pin | Connection | Function |
| :--- | :--- | :--- | :--- |
| **Pin 1** | GP0 | ESP-01 RXD / Serial TX | Pico → ESP Serial TX |
| **Pin 2** | GP1 | ESP-01 TXD / Serial RX | ESP → Pico Serial RX |
| **Pin 4** | GP2 | ESP-01 RST | Hardware Reset |
| **Pin 5** | GP3 | ESP-01 IO0 | Boot / Flash Mode |
| **Pin 6** | GP4 | OLED SDA | I2C0 Data Line |
| **Pin 7** | GP5 | OLED SCL | I2C0 Clock Line |
| **Pin 9** | GP6 | ESP-01 CH_PD | Chip Enable / Sleep Control |
| **Pin 36** | 3V3 (OUT) | OLED + ESP-01 VCC | 3.3V Power Output |
| **Pin 38** | GND | Common GND | Ground |
| **Pin 39** | VSYS | Power Input | 5V Power / Switch |

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
 Developed for Raspberry Pi Pico + ESP8266 + SSD1306 OLED
 Apps & Tools: fetch | tmux | htop | calc | nano | service list
 Type 'help' for command reference or 'tmux help' for split-screen guide.

root@pico:/# fetch
```

---

## 📜 License
Licensed under the MIT License.
