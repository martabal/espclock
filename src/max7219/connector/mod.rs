//! A driver to interface with the max7219 (LED matrix display driver)
//!
//! This driver was built using [`esp-hal`] peripherals.
//!
//! [`esp-hal`]: https://docs.rs/esp-hal

#![deny(unsafe_code)]
#![deny(warnings)]

use esp_hal::gpio::Output;
use esp_hal::spi::master::Spi;

pub mod device;

use device::{Connector, PinConnector, SpiConnector, SpiConnectorSW};

use crate::max7219::draw::chars::Glyph;

/// Maximum number of displays connected in series.
pub const MAX_DISPLAYS: usize = 4;

/// Number of columns per device.
pub const NB_COLUMNS: usize = 8;

/// Number of lines per device.
pub const NB_LINES: usize = 8;

/// Number of lines per device.
pub const TOTAL_PIXEL_PER_DEVICE: usize = NB_LINES * NB_COLUMNS;

/// Number of lines per device.
pub const TOTAL_PIXEL: usize = TOTAL_PIXEL_PER_DEVICE * MAX_DISPLAYS;

/// Possible command register values on the display chip.
#[derive(Clone, Copy)]
pub enum Command {
    Noop = 0x00,
    Digit0 = 0x01,
    Digit1 = 0x02,
    Digit2 = 0x03,
    Digit3 = 0x04,
    Digit4 = 0x05,
    Digit5 = 0x06,
    Digit6 = 0x07,
    Digit7 = 0x08,
    DecodeMode = 0x09,
    Intensity = 0x0A,
    ScanLimit = 0x0B,
    Power = 0x0C,
    DisplayTest = 0x0F,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandError {
    InvalidDigit,
}

impl Command {
    /// Convert register to u8 value
    #[must_use]
    pub const fn addr(self) -> u8 {
        self as u8
    }

    /// Try to convert a digit index (0-7) into a corresponding `Register::DigitN`.
    #[allow(unused)]
    pub(crate) const fn try_digit(digit: u8) -> Result<Self, CommandError> {
        match digit {
            0 => Ok(Self::Digit0),
            1 => Ok(Self::Digit1),
            2 => Ok(Self::Digit2),
            3 => Ok(Self::Digit3),
            4 => Ok(Self::Digit4),
            5 => Ok(Self::Digit5),
            6 => Ok(Self::Digit6),
            7 => Ok(Self::Digit7),
            _ => Err(CommandError::InvalidDigit),
        }
    }

    /// Returns an iterator over all digit registers (Digit0 to Digit7).
    ///
    /// Useful for iterating through display rows or columns when writing
    /// to all digits of a MAX7219 device in order.
    pub fn digits() -> impl Iterator<Item = Self> {
        [
            Self::Digit0,
            Self::Digit1,
            Self::Digit2,
            Self::Digit3,
            Self::Digit4,
            Self::Digit5,
            Self::Digit6,
            Self::Digit7,
        ]
        .into_iter()
    }
}

#[derive(Copy, Clone, Debug)]
pub enum Intensity {
    Min = 0x00,
    VeryLow = 0x02,
    Low = 0x04,
    Medium = 0x07,
    MediumHigh = 0x09,
    High = 0x0B,
    VeryHigh = 0x0D,
    Max = 0x0F,
}

/// Decode modes for BCD encoded input.
#[derive(Copy, Clone)]
pub enum DecodeMode {
    NoDecode = 0x00,
    CodeBDigit0 = 0x01,
    CodeBDigits3_0 = 0x0F,
    CodeBDigits7_0 = 0xFF,
}

///
/// Error raised in case there was an error
/// during communication with the max7219 chip.
///
#[derive(Debug)]
pub enum DataError {
    /// An error occurred when working with SPI
    Spi,
    /// An error occurred when working with a PIN
    Pin,
    /// An error occurred when cating values
    ConversionError,
}

#[derive(PartialEq, Eq)]
pub enum Direction {
    TopBottom,
    BottomTop,
}
pub struct Max7219<CONNECTOR> {
    c: CONNECTOR,
    decode_mode: DecodeMode,
    direction: Direction,
}

impl<CONNECTOR> Max7219<CONNECTOR>
where
    CONNECTOR: Connector,
{
    pub fn power_on(&mut self) -> Result<(), DataError> {
        for i in 0..self.c.devices() {
            self.c.write_data(i, Command::Power, 0x01)?;
        }
        Ok(())
    }

    pub fn power_off(&mut self) -> Result<(), DataError> {
        for i in 0..self.c.devices() {
            self.c.write_data(i, Command::Power, 0x00)?;
        }
        Ok(())
    }

    pub fn clear_display(&mut self, addr: usize) -> Result<(), DataError> {
        for i in 1..9 {
            self.c.write_raw(addr, i, 0x00)?;
        }
        Ok(())
    }

    pub fn set_global_intensity(&mut self, intensity: Intensity) -> Result<(), DataError> {
        for i in 0..self.c.devices() {
            self.c.write_data(i, Command::Intensity, intensity as u8)?;
        }

        Ok(())
    }

    pub fn set_intensity(&mut self, addr: usize, intensity: Intensity) -> Result<(), DataError> {
        self.c.write_data(addr, Command::Intensity, intensity as u8)
    }

    pub fn set_decode_mode(&mut self, addr: usize, mode: DecodeMode) -> Result<(), DataError> {
        self.decode_mode = mode;
        self.c.write_data(addr, Command::DecodeMode, mode as u8)
    }

    pub fn draw_glyphs(&mut self, glyphs: &[Glyph]) -> Result<(), DataError> {
        let device_count = self.c.devices();

        let mut row_data = [[0u8; MAX_DISPLAYS]; 8];
        let mut device_index = 0usize;
        let mut col_offset = 0usize;

        for glyph in glyphs {
            if device_index >= device_count {
                break;
            }
            let cols: &[u8] = glyph.into();

            for &col_byte in cols {
                if device_index >= device_count {
                    break;
                }

                if col_offset >= 8 {
                    device_index += 1;
                    col_offset = 0;
                    if device_index >= device_count {
                        break;
                    }
                }

                let bit_pos = if self.direction == Direction::TopBottom {
                    7 - col_offset
                } else {
                    col_offset
                };

                for (row, row_item) in row_data.iter_mut().enumerate() {
                    let applied_row = if self.direction == Direction::TopBottom {
                        7 - row
                    } else {
                        row
                    };
                    let lit = (col_byte >> (applied_row)) & 1;
                    if lit != 0 {
                        row_item[device_index] |= 1 << bit_pos;
                    }
                }
                col_offset += 1;
            }
        }

        let used_devices = (device_index + 1).min(device_count);

        for (row_index, digit_register) in Command::digits().enumerate() {
            for (dev, _) in row_data.iter().enumerate().take(used_devices) {
                let hw_dev = if self.direction == Direction::TopBottom {
                    dev
                } else {
                    used_devices - 1 - dev
                };

                self.c
                    .write_raw(hw_dev, digit_register.addr(), row_data[row_index][dev])?;
            }
        }

        Ok(())
    }

    pub fn test(&mut self, addr: usize, is_on: bool) -> Result<(), DataError> {
        if is_on {
            self.c.write_data(addr, Command::DisplayTest, 0x01)
        } else {
            self.c.write_data(addr, Command::DisplayTest, 0x00)
        }
    }

    pub fn new(connector: CONNECTOR, direction: Direction) -> Result<Self, DataError> {
        let mut max7219 = Self {
            c: connector,
            decode_mode: DecodeMode::NoDecode,
            direction,
        };
        max7219.init()?;
        Ok(max7219)
    }

    pub fn init(&mut self) -> Result<(), DataError> {
        for i in 0..self.c.devices() {
            self.test(i, false)?;
            self.c.write_data(i, Command::ScanLimit, 0x07)?;
            self.set_decode_mode(i, DecodeMode::NoDecode)?;
            self.clear_display(i)?;
        }
        self.power_off()?;
        Ok(())
    }
}

// --- Constructors ---

impl<'d> Max7219<PinConnector<'d>> {
    ///
    /// Construct a new max7219 driver instance from DATA, CS and SCK pins.
    ///
    /// # Arguments
    ///
    /// * `displays` - number of displays connected in series
    /// * `data` - the MOSI/DATA pin set to output mode
    /// * `cs`   - the CS (LOAD) pin set to output mode
    /// * `sck`  - the SCK clock pin set to output mode
    ///
    pub fn from_pins(
        displays: usize,
        data: Output<'d>,
        cs: Output<'d>,
        sck: Output<'d>,
    ) -> Result<Self, DataError> {
        Max7219::new(
            PinConnector::new(displays, data, cs, sck),
            Direction::TopBottom,
        )
    }
}

impl<'d> Max7219<SpiConnector<'d>> {
    ///
    /// Construct a new max7219 driver instance from pre-existing SPI
    /// in full hardware mode. CS is managed by the SPI peripheral.
    ///
    /// * `NOTE` - initialise SPI in `MODE_0`, max 10 MHz.
    ///
    /// # Arguments
    ///
    /// * `displays` - number of displays connected in series
    /// * `spi`      - the SPI interface (MOSI, MISO unused, CLK)
    ///
    pub fn from_spi(
        displays: usize,
        spi: Spi<'d, esp_hal::Blocking>,
        direction: Direction,
    ) -> Result<Self, DataError> {
        Max7219::new(SpiConnector::new(displays, spi), direction)
    }
}

impl<'d> Max7219<SpiConnectorSW<'d>> {
    ///
    /// Construct a new max7219 driver instance from pre-existing SPI
    /// and a manually controlled CS pin.
    ///
    /// * `NOTE` - initialise SPI in `MODE_0`, max 10 MHz.
    ///
    /// # Arguments
    ///
    /// * `displays` - number of displays connected in series
    /// * `spi`      - the SPI interface (MOSI, MISO unused, CLK)
    /// * `cs`       - the CS (LOAD) pin set to output mode
    ///
    pub fn from_spi_cs(
        displays: usize,
        spi: Spi<'d, esp_hal::Blocking>,
        cs: Output<'d>,
        direction: Direction,
    ) -> Result<Self, DataError> {
        Max7219::new(SpiConnectorSW::new(displays, spi, cs), direction)
    }
}
