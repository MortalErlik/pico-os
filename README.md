# Pico OS: Custom Bare-Metal OS in Rust & ARM Assembly

Pico OS is a lightweight, bare-metal multitasking operating system designed and implemented from scratch for the **Raspberry Pi Pico (RP2040 Cortex-M0+)**, featuring custom memory management, an interactive Unix-like shell, an in-memory/flash Virtual File System (VFS), a preemptive task scheduler with Assembly context switching, an interactive live process monitor (`htop`), a full-screen text editor (`nano`), and hardware peripheral drivers for **SSD1306 OLED (I2C0)** and **ESP8266 ESP-01 (UART0)**.

---

## Architecture & Features

### 1. Custom RAM Management (`src/mm/`)
- Custom first-fit linked-list heap allocator implementing Rust's `GlobalAlloc` trait.
- Automatic contiguous block coalescing upon deallocation to eliminate memory fragmentation.
- Real-time memory allocation tracking: Total RAM, Allocated Bytes, Free Bytes, Peak Memory, and Allocation Count.
- Accessible via the `free` command and `htop` dashboard.

### 2. Preemptive Multitasking & Context Switching (`src/task/`)
- ARM Cortex-M0+ assembly context switcher utilizing the `PendSV` interrupt handler.
- Context save/restore for low (`r4-r7`) and high (`r8-r11`) registers using Thumb-1 instructions.
- Round-Robin preemptive scheduler triggered by the 1000 Hz `SysTick` timer interrupt.
- Task Control Block (TCB) with PID, state (`Ready`, `Running`, `Sleeping`, `Blocked`, `Dead`), stack usage tracking, and CPU cycle percentage.
- Task lifecycle management: `spawn`, `sleep_ms`, `yield_now`, and `kill`.

### 3. Virtual File System & Linux Commands (`src/fs/`, `src/shell/`)
- Hierarchical Unix-style directory tree (`/`, `/bin`, `/etc`, `/home`, `/dev`, `/proc`).
- Built-in Linux commands:
  - **File Operations**: `ls`, `cd`, `pwd`, `mkdir`, `rm` (`-r`), `touch`, `cat`, `cp`, `mv`, `echo` (with `>` and `>>` redirection support).
  - **Process & System**: `ps`, `kill`, `spawn`, `free`, `uptime`, `uname` (`-a`), `whoami`, `clear`, `reboot`.
  - **Hardware Control**: `pin` (read/set/clear/toggle GPIOs), `i2c_scan` (I2C bus scanner).

### 4. Interactive Live Process Monitor: `htop` (`src/htop/`)
- ANSI color terminal dashboard.
- Live graphical bar meters for CPU usage (Core 0) and RAM usage (Used vs Total).
- Real-time task table showing PID, User, State, CPU%, Stack Depth, Max Stack, and Ticks.
- Interactive keyboard controls:
  - Press `k` or `K` to trigger interactive PID kill mode (`Signal SIGKILL`).
  - Press `q`, `Q`, or `Esc` to cleanly exit back to the shell.

### 5. Interactive Full-Screen Text Editor: `nano` (`src/editor/`)
- VT100/ANSI full-screen terminal text editor.
- Inverse-video header bar showing current filename and `[Modified]` indicator.
- Dynamic scrolling, arrow key cursor navigation (`Up`, `Down`, `Left`, `Right`), backspace, and newline insertion.
- Keyboard shortcuts:
  - `Ctrl+O`: Save (WriteOut) buffer to filesystem.
  - `Ctrl+K`: Cut line.
  - `Ctrl+X`: Exit back to shell.

### 6. Hardware Peripheral Support (`src/drivers/`)
- **Dual Terminal I/O**: Shell is simultaneously active over USB CDC-ACM Serial (Micro-USB) and Hardware UART0 (GP0 TX / GP1 RX).
- **SSD1306 OLED Display (I2C0 GP4/GP5)**: Real-time graphical dashboard showing CPU%, RAM usage bar, Task count, and Uptime.
- **ESP-01 (ESP8266) Management**: Power, reset, and boot pin control (`GP2 RST`, `GP3 IO0`, `GP6 CH_PD`).

---

## Pinout Map

| Physical Pin | Pico Pin | Connection | Function |
| :--- | :--- | :--- | :--- |
| **Pin 1** | GP0 | ESP-01 RXD / Serial TX | Pico → ESP Veri Gönderimi |
| **Pin 2** | GP1 | ESP-01 TXD / Serial RX | ESP → Pico Veri Alımı |
| **Pin 4** | GP2 | ESP-01 RST | Donanımsal Reset |
| **Pin 5** | GP3 | ESP-01 IO0 | Boot / Flaşlama Seçimi |
| **Pin 6** | GP4 | OLED SDA | I2C0 Veri Hattı |
| **Pin 7** | GP5 | OLED SCL | I2C0 Saat Hattı |
| **Pin 9** | GP6 | ESP-01 CH_PD | Çip Etkinleştirme / Uyku Kontrolü |
| **Pin 36** | 3V3 (OUT) | OLED + ESP-01 VCC | 3.3V Güç Beslemesi |
| **Pin 38** | GND | Ortak GND | Sistem Toprak Hattı |
| **Pin 39** | VSYS | Güç Girişi + 1000 µF | Switch Çıkışı |

---

## How to Flash to Raspberry Pi Pico

1. Hold down the **BOOTSEL** button on your Raspberry Pi Pico and plug it into your computer via USB.
2. The Pico will appear as a mass storage drive named `RPI-RP2`.
3. Copy/Drag-and-drop the generated `pico_os.uf2` file onto the `RPI-RP2` drive:
   ```bash
   cp pico_os.uf2 /media/$USER/RPI-RP2/
   ```
4. The Pico will automatically reboot and start Pico OS!

---

## Connecting to the Interactive Shell

Connect via any serial terminal (e.g. PuTTY, minicom, screen, or Arduino Serial Monitor) at **115200 baud**:
```bash
# Linux:
picocom -b 115200 /dev/ttyACM0
# or
screen /dev/ttyACM0 115200
```
