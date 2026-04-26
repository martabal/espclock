use esp_hal::gpio::Output;
use esp_hal::spi::master::Spi;

use super::{Command, DataError, MAX_DISPLAYS};

/// Describes the interface used to connect to the MAX7219
pub trait Connector {
    fn devices(&self) -> usize;

    fn write_data(&mut self, addr: usize, command: Command, data: u8) -> Result<(), DataError> {
        self.write_raw(addr, command as u8, data)
    }

    fn write_raw(&mut self, addr: usize, header: u8, data: u8) -> Result<(), DataError>;
}

/// Direct GPIO pins connector
pub struct PinConnector<'d> {
    devices: usize,
    buffer: [u8; MAX_DISPLAYS * 2],
    data: Output<'d>,
    cs: Output<'d>,
    sck: Output<'d>,
}

impl<'d> PinConnector<'d> {
    pub(crate) const fn new(
        displays: usize,
        data: Output<'d>,
        cs: Output<'d>,
        sck: Output<'d>,
    ) -> Self {
        PinConnector {
            devices: displays,
            buffer: [0; MAX_DISPLAYS * 2],
            data,
            cs,
            sck,
        }
    }
}

impl Connector for PinConnector<'_> {
    fn devices(&self) -> usize {
        self.devices
    }

    fn write_raw(&mut self, addr: usize, header: u8, data: u8) -> Result<(), DataError> {
        let offset = addr * 2;
        let max_bytes = self.devices * 2;
        self.buffer = [0; MAX_DISPLAYS * 2];

        self.buffer[offset] = header;
        self.buffer[offset + 1] = data;

        self.cs.set_low();
        for b in 0..max_bytes {
            let value = self.buffer[b];
            for i in 0..8 {
                if value & (1 << (7 - i)) > 0 {
                    self.data.set_high();
                } else {
                    self.data.set_low();
                }
                self.sck.set_high();
                self.sck.set_low();
            }
        }
        self.cs.set_high();

        Ok(())
    }
}

/// Hardware SPI connector (CS managed by hardware/SPI peripheral)
pub struct SpiConnector<'d> {
    devices: usize,
    buffer: [u8; MAX_DISPLAYS * 2],
    spi: Spi<'d, esp_hal::Blocking>,
}

impl<'d> SpiConnector<'d> {
    pub(crate) const fn new(displays: usize, spi: Spi<'d, esp_hal::Blocking>) -> Self {
        SpiConnector {
            devices: displays,
            buffer: [0; MAX_DISPLAYS * 2],
            spi,
        }
    }
}

impl Connector for SpiConnector<'_> {
    fn devices(&self) -> usize {
        self.devices
    }

    fn write_raw(&mut self, addr: usize, header: u8, data: u8) -> Result<(), DataError> {
        let offset = addr * 2;
        let max_bytes = self.devices * 2;
        self.buffer = [0; MAX_DISPLAYS * 2];

        self.buffer[offset] = header;
        self.buffer[offset + 1] = data;

        self.spi
            .write(&self.buffer[0..max_bytes])
            .map_err(|_| DataError::Spi)?;

        Ok(())
    }
}

/// Software-controlled CS connector with SPI transfer
pub struct SpiConnectorSW<'d> {
    spi_c: SpiConnector<'d>,
    cs: Output<'d>,
}

impl<'d> SpiConnectorSW<'d> {
    pub(crate) const fn new(
        displays: usize,
        spi: Spi<'d, esp_hal::Blocking>,
        cs: Output<'d>,
    ) -> Self {
        SpiConnectorSW {
            spi_c: SpiConnector::new(displays, spi),
            cs,
        }
    }
}

impl Connector for SpiConnectorSW<'_> {
    fn devices(&self) -> usize {
        self.spi_c.devices
    }

    fn write_raw(&mut self, addr: usize, header: u8, data: u8) -> Result<(), DataError> {
        self.cs.set_low();
        self.spi_c
            .write_raw(addr, header, data)
            .map_err(|_| DataError::Spi)?;
        self.cs.set_high();

        Ok(())
    }
}
