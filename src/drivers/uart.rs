//! UART Driver for Pico OS (GP0 TX / GP1 RX)
//! Communicates with the ESP-01 module or serial console.

use embedded_hal::digital::v2::OutputPin;
use rp2040_hal::gpio::bank0::{Gpio0, Gpio1, Gpio2, Gpio3, Gpio6};
use rp2040_hal::gpio::{FunctionSio, FunctionUart, Pin, PullDown, PullNone, SioOutput};
use rp2040_hal::pac::UART0;
use rp2040_hal::uart::UartPeripheral;

pub type EspPin<I> = Pin<I, FunctionSio<SioOutput>, PullDown>;

pub struct EspControlPins {
    pub rst: EspPin<Gpio2>,
    pub io0: EspPin<Gpio3>,
    pub ch_pd: EspPin<Gpio6>,
}

impl EspControlPins {
    pub fn enable_esp(&mut self) {
        let _ = self.ch_pd.set_high();
        let _ = self.io0.set_high(); // Normal boot mode
        let _ = self.rst.set_high();
    }

    pub fn reset_esp(&mut self) {
        let _ = self.rst.set_low();
        cortex_m::asm::delay(1_000_000);
        let _ = self.rst.set_high();
    }
}

pub type UartPins = (Pin<Gpio0, FunctionUart, PullNone>, Pin<Gpio1, FunctionUart, PullNone>);
pub type UartDevice = UartPeripheral<rp2040_hal::uart::Enabled, UART0, UartPins>;

pub struct UartDriver {
    pub uart: Option<UartDevice>,
}

impl UartDriver {
    pub const fn new() -> Self {
        UartDriver { uart: None }
    }

    pub fn write_str(&mut self, s: &str) {
        if let Some(ref mut uart) = self.uart {
            let _ = uart.write_full_blocking(s.as_bytes());
        }
    }

    pub fn write_byte(&mut self, byte: u8) {
        if let Some(ref mut uart) = self.uart {
            let _ = uart.write_full_blocking(&[byte]);
        }
    }

    pub fn read_byte(&mut self) -> Option<u8> {
        if let Some(ref mut uart) = self.uart {
            let mut buf = [0u8; 1];
            match uart.read_raw(&mut buf) {
                Ok(1) => Some(buf[0]),
                _ => None,
            }
        } else {
            None
        }
    }
}
