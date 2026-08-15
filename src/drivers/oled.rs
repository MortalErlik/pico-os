//! SSD1306 OLED Display Driver on I2C0 (GP4 SDA / GP5 SCL)
//! Renders real-time system stats, logo, and active process information.

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{PrimitiveStyle, Rectangle},
    text::{Baseline, Text},
};
use rp2040_hal::gpio::bank0::{Gpio4, Gpio5};
use rp2040_hal::gpio::{FunctionI2C, Pin, PullUp};
use rp2040_hal::i2c::I2C;
use rp2040_hal::pac::I2C0;
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

pub type I2cPins = (Pin<Gpio4, FunctionI2C, PullUp>, Pin<Gpio5, FunctionI2C, PullUp>);
pub type I2cBus = I2C<I2C0, I2cPins>;
pub type OledDisplay = Ssd1306<I2CInterface<I2cBus>, DisplaySize128x64, ssd1306::mode::BufferedGraphicsMode<DisplaySize128x64>>;

pub struct OledDriver {
    pub display: Option<OledDisplay>,
}

impl OledDriver {
    pub const fn new() -> Self {
        OledDriver { display: None }
    }

    pub fn init(&mut self, i2c: I2cBus) {
        let interface = I2CDisplayInterface::new(i2c);
        let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();

        if display.init().is_ok() {
            let _ = display.clear(BinaryColor::Off);
            let _ = display.flush();
            self.display = Some(display);
        }
    }

    pub fn draw_dashboard(&mut self, cpu_pct: u8, ram_used_kb: usize, ram_total_kb: usize, task_count: usize, uptime_secs: u32) {
        if let Some(ref mut display) = self.display {
            let _ = display.clear(BinaryColor::Off);

            let style = MonoTextStyleBuilder::new()
                .font(&FONT_6X10)
                .text_color(BinaryColor::On)
                .build();

            // Header
            let _ = Text::with_baseline("=== PICO OS 1.0 ===", Point::new(0, 0), style, Baseline::Top).draw(display);

            // CPU Bar
            let mut cpu_buf = heapless::String::<32>::new();
            let _ = core::fmt::write(&mut cpu_buf, format_args!("CPU: {:>3}%", cpu_pct));
            let _ = Text::with_baseline(&cpu_buf, Point::new(0, 16), style, Baseline::Top).draw(display);

            let cpu_bar_width = ((cpu_pct as u32 * 60) / 100).min(60);
            let _ = Rectangle::new(Point::new(64, 17), Size::new(60, 8))
                .into_styled(PrimitiveStyle::with_stroke(BinaryColor::On, 1))
                .draw(display);
            if cpu_bar_width > 2 {
                let _ = Rectangle::new(Point::new(65, 18), Size::new(cpu_bar_width.saturating_sub(2), 6))
                    .into_styled(PrimitiveStyle::with_fill(BinaryColor::On))
                    .draw(display);
            }

            // RAM Bar
            let mut ram_buf = heapless::String::<32>::new();
            let _ = core::fmt::write(&mut ram_buf, format_args!("RAM: {}/{}K", ram_used_kb, ram_total_kb));
            let _ = Text::with_baseline(&ram_buf, Point::new(0, 30), style, Baseline::Top).draw(display);

            // Tasks & Uptime
            let mut task_buf = heapless::String::<32>::new();
            let _ = core::fmt::write(&mut task_buf, format_args!("TASKS: {:<2} UP: {}s", task_count, uptime_secs));
            let _ = Text::with_baseline(&task_buf, Point::new(0, 44), style, Baseline::Top).draw(display);

            // Bottom status
            let _ = Text::with_baseline("Status: RUNNING", Point::new(0, 54), style, Baseline::Top).draw(display);

            let _ = display.flush();
        }
    }

    pub fn draw_text(&mut self, line1: &str, line2: &str, line3: &str) {
        if let Some(ref mut display) = self.display {
            let _ = display.clear(BinaryColor::Off);
            let style = MonoTextStyleBuilder::new()
                .font(&FONT_6X10)
                .text_color(BinaryColor::On)
                .build();

            let _ = Text::with_baseline(line1, Point::new(0, 5), style, Baseline::Top).draw(display);
            let _ = Text::with_baseline(line2, Point::new(0, 25), style, Baseline::Top).draw(display);
            let _ = Text::with_baseline(line3, Point::new(0, 45), style, Baseline::Top).draw(display);
            let _ = display.flush();
        }
    }
}
