//! USB CDC Serial Driver for Pico OS

use usb_device::class_prelude::*;
use usb_device::prelude::*;
use usbd_serial::{SerialPort, USB_CLASS_CDC};

pub struct UsbDriver {
    pub dev: UsbDevice<'static, rp2040_hal::usb::UsbBus>,
    pub serial: SerialPort<'static, rp2040_hal::usb::UsbBus>,
}

impl UsbDriver {
    pub fn new(bus: &'static UsbBusAllocator<rp2040_hal::usb::UsbBus>) -> Self {
        let serial = SerialPort::new(bus);
        let dev = UsbDeviceBuilder::new(bus, UsbVidPid(0x16c0, 0x27dd))
            .device_class(USB_CLASS_CDC)
            .build();

        UsbDriver { dev, serial }
    }

    pub fn poll(&mut self) -> bool {
        self.dev.poll(&mut [&mut self.serial])
    }

    pub fn write_str(&mut self, text: &str) {
        let bytes = text.as_bytes();
        let mut offset = 0;
        while offset < bytes.len() {
            let chunk = &bytes[offset..];
            match self.serial.write(chunk) {
                Ok(count) => offset += count,
                Err(UsbError::WouldBlock) => {
                    self.poll();
                }
                Err(_) => break,
            }
        }
    }

    pub fn read(&mut self, buf: &mut [u8]) -> Result<usize, usb_device::UsbError> {
        self.serial.read(buf)
    }
}
