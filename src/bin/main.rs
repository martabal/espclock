#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use esp_hal::clock::CpuClock;
use esp_hal::main;

use esp_hal::spi::master::Spi;
use esp_hal::time::{Duration, Instant, Rate};
use esp_hal::timer::timg::TimerGroup;

use defmt_rtt as _;
use esp_radio::wifi::ap::AccessPointConfig;
use esp_radio::wifi::{Config, ControllerConfig};
use espclock::max7219::connector::{Direction, Intensity, MAX_DISPLAYS, Max7219, NB_COLUMNS};
use espclock::max7219::draw::chars::{Digit, Glyph};
use espclock::println;

#[panic_handler]
const fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

extern crate alloc;

// This creates a default app-descriptor required by the esp-idf bootloader.
// For more information see: <https://docs.espressif.com/projects/esp-idf/en/stable/esp32/api-reference/system/app_image_format.html#application-description>
esp_bootloader_esp_idf::esp_app_desc!();

#[allow(
    clippy::large_stack_frames,
    reason = "it's not unusual to allocate larger buffers etc. in main"
)]
#[main]
fn main() -> ! {
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    let peripherals = esp_hal::init(config);

    esp_alloc::heap_allocator!(#[esp_hal::ram(reclaimed)] size: 66320);

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let sw_interrupt =
        esp_hal::interrupt::software::SoftwareInterruptControl::new(peripherals.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_interrupt.software_interrupt0);

    let (mut wifi_controller, _interfaces) =
        esp_radio::wifi::new(peripherals.WIFI, ControllerConfig::default())
            .expect("Failed to initialize Wi-Fi controller");

    let ap_config = Config::AccessPoint(AccessPointConfig::default());

    wifi_controller.set_config(&ap_config).unwrap();

    let sclk = peripherals.GPIO4;
    let cs = peripherals.GPIO10;
    let mosi = peripherals.GPIO6;

    let spi_config = esp_hal::spi::master::Config::default()
        .with_frequency(Rate::from_mhz(10u32))
        .with_mode(esp_hal::spi::Mode::_0);

    let spi = Spi::new(peripherals.SPI2, spi_config)
        .unwrap()
        .with_sck(sclk)
        .with_mosi(mosi)
        .with_cs(cs);

    let mut display = Max7219::from_spi(4, spi, Direction::TopBottom).unwrap();
    display.power_on().unwrap();

    let glyphs: &[Glyph] = &[
        Glyph::Digit(Digit::One),
        Glyph::Space,
        Glyph::Digit(Digit::Two),
        Glyph::Space,
        Glyph::Digit(Digit::Three),
        Glyph::Space,
        Glyph::Digit(Digit::Four),
        Glyph::Space,
        Glyph::Digit(Digit::Five),
        Glyph::Space,
        Glyph::Digit(Digit::Six),
        Glyph::Space,
        Glyph::Digit(Digit::Seven),
        Glyph::Space,
        Glyph::Digit(Digit::Eight),
    ];

    let width: usize = glyphs.iter().map(Glyph::width).sum();

    assert!(width < MAX_DISPLAYS * NB_COLUMNS);

    display.draw_glyphs(glyphs).unwrap();

    println!("Debug print");

    display.draw_glyphs(glyphs).unwrap();

    display.set_global_intensity(Intensity::Min).unwrap();

    loop {
        let delay_start = Instant::now();
        while delay_start.elapsed() < Duration::from_millis(500) {}
    }
}
