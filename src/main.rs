//! Pico OS: Custom Bare-Metal Operating System in Rust & Assembly
//! Target: Raspberry Pi Pico (RP2040 Cortex-M0+ Dual-Core SMP)

#![no_std]
#![no_main]

extern crate alloc;

pub mod calc;
pub mod drivers;
pub mod editor;
pub mod fs;
pub mod htop;
pub mod mm;
pub mod shell;
pub mod task;
pub mod tmux;

use cortex_m_rt::entry;
use embedded_hal::digital::v2::OutputPin;
use panic_halt as _;
use rp2040_hal::clocks::{init_clocks_and_plls, Clock};
use rp2040_hal::fugit::RateExtU32;
use rp2040_hal::gpio::{FunctionUart, Pins, PullNone};
use rp2040_hal::multicore::{Multicore, Stack};
use rp2040_hal::pac;
use rp2040_hal::sio::Sio;
use rp2040_hal::uart::{DataBits, StopBits, UartConfig, UartPeripheral};
use rp2040_hal::usb::UsbBus;
use rp2040_hal::watchdog::Watchdog;
use usb_device::class_prelude::UsbBusAllocator;
use usb_device::prelude::*;
use usbd_serial::{SerialPort, USB_CLASS_CDC};

use drivers::uart::EspControlPins;
use shell::Shell;

/// Boot2 sector for RP2040 QSPI Flash (W25Q080 / standard Winbond Flash)
#[link_section = ".boot2"]
#[used]
pub static BOOT2_FIRMWARE: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

static mut ESP_PINS: Option<EspControlPins> = None;
static mut CORE1_STACK: Stack<4096> = Stack::new();

#[entry]
fn main() -> ! {
    // 1. Initialize RAM Heap (216 KB for OS memory management)
    mm::init_heap();

    // 2. Initialize Hardware Peripherals & Clocks
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let mut sio = Sio::new(pac.SIO);

    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // 3. Turn ON onboard Green LED (GP25) as visual power & boot confirmation!
    let mut led_pin = pins.gpio25.into_push_pull_output();
    let _ = led_pin.set_high();

    // 4. Configure UART0 (GP0 TX, GP1 RX for ESP-01 / External Serial)
    let uart_pins = (
        pins.gpio0.into_function::<FunctionUart>().into_pull_type::<PullNone>(),
        pins.gpio1.into_function::<FunctionUart>().into_pull_type::<rp2040_hal::gpio::PullUp>(),
    );
    let mut uart = UartPeripheral::new(pac.UART0, uart_pins, &mut pac.RESETS)
        .enable(
            UartConfig::new(115200.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

    // 5. Configure ESP-01 Control Pins (GP2 RST, GP3 IO0, GP6 CH_PD)
    let mut esp_rst = pins.gpio2.into_push_pull_output();
    let mut esp_io0 = pins.gpio3.into_push_pull_output();
    let mut esp_ch_pd = pins.gpio6.into_push_pull_output();
    let _ = esp_ch_pd.set_high();
    let _ = esp_io0.set_high();
    let _ = esp_rst.set_high();

    unsafe {
        ESP_PINS = Some(EspControlPins {
            rst: esp_rst,
            io0: esp_io0,
            ch_pd: esp_ch_pd,
        });
    }

    // 6. Initialize File System & Scheduler (All RAM allocations done before USB!)
    fs::init_fs();
    task::init_scheduler();

    // 7. Configure SysTick Timer for 1ms System Ticks on Core 0
    let mut syst = core.SYST;
    syst.set_clock_source(cortex_m::peripheral::syst::SystClkSource::Core);
    syst.set_reload(125_000 - 1);
    syst.clear_current();
    syst.enable_counter();
    syst.enable_interrupt();

    // 8. Initialize Core 1 as Independent Real-Time Multiprocessing Worker!
    let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);
    let cores = mc.cores();
    let core1 = &mut cores[1];
    let _ = core1.spawn(unsafe { &mut CORE1_STACK.mem }, move || {
        core1_task();
    });

    // Notify flash driver that Core 1 is running
    fs::flash::CORE1_SPAWNED.store(true, core::sync::atomic::Ordering::SeqCst);

    // 9. Configure USB CDC-ACM Serial Device (Right before loop to ensure instant poll response!)
    let usb_bus = UsbBusAllocator::new(UsbBus::new(
        pac.USBCTRL_REGS,
        pac.USBCTRL_DPRAM,
        clocks.usb_clock,
        true,
        &mut pac.RESETS,
    ));

    let mut serial = SerialPort::new(&usb_bus);
    let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
        .device_class(USB_CLASS_CDC)
        .build();

    let mut shell = Shell::new();
    let mut banner_printed = false;
    let mut last_htop_update = task::get_uptime_ticks();

    // Kernel Main / Core 0 Terminal Dispatch Loop
    loop {
        let mut did_work = false;

        // Helper to safely write data longer than 64 bytes to USB without freezing
        macro_rules! write_out {
            ($s:expr) => {{
                let _ = uart.write_raw($s.as_bytes());
                if usb_dev.state() == usb_device::device::UsbDeviceState::Configured {
                    let mut b = $s.as_bytes();
                    let mut retries = 0;
                    while !b.is_empty() && retries < 10000 {
                        match serial.write(b) {
                            Ok(n) if n > 0 => {
                                b = &b[n..];
                                retries = 0;
                            }
                            _ => {
                                let _ = usb_dev.poll(&mut [&mut serial]);
                                retries += 1;
                            }
                        }
                    }
                }
            }};
        }

        // Poll USB Device (Processes USB packet events)
        if usb_dev.poll(&mut [&mut serial]) {
            if !banner_printed {
                shell.print_banner(|s| write_out!(s));
                banner_printed = true;
                did_work = true;
            }

            let mut buf = [0u8; 64];
            match serial.read(&mut buf) {
                Err(_) | Ok(0) => {}
                Ok(count) => {
                    did_work = true;
                    for i in 0..count {
                        shell.handle_byte(buf[i], |s| write_out!(s));
                    }
                }
            }
        }

        // Live Htop Auto-refresh (Every 1 second)
        let current_ticks = task::get_uptime_ticks();
        if current_ticks.wrapping_sub(last_htop_update) >= 1000 {
            last_htop_update = current_ticks;
            shell.tick(|s| write_out!(s));
        }

        // Automatic Delayed Writeback (Kernel Background Auto-Sync)
        if fs::poll_auto_sync(current_ticks) {
            did_work = true;
        }

        // Read UART characters (from ESP-01 or external serial)
        let mut uart_buf = [0u8; 1];
        if let Ok(1) = uart.read_raw(&mut uart_buf) {
            did_work = true;
            shell.handle_byte(uart_buf[0], |s| write_out!(s));
        }

        // Report Core 0 activity
        task::report_core0_tick(did_work);
    }
}

/// Core 1 Real-Time Multiprocessing Worker Entry Point
fn core1_task() -> ! {
    loop {
        // Safely check and handle flash write lockouts while executing from RAM
        fs::flash::core1_check_flash_lockout();

        // Track real-time activity for Core 1 (idle when waiting)
        task::report_core1_tick(false);

        // Real-time task work or background loop (10ms)
        for _ in 0..100 {
            fs::flash::core1_check_flash_lockout();
            cortex_m::asm::delay(12_500); // 100 microseconds delay
        }
    }
}

/// SysTick Exception Handler for Preemptive Scheduling & Clock Ticks
#[no_mangle]
pub extern "C" fn SysTick() {
    task::tick_clock();
}
