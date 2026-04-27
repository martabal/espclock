#![no_std]

pub mod max7219;

#[cfg(debug_assertions)]
#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {
        esp_println::println!($($arg)*)
    };
}

#[cfg(not(debug_assertions))]
#[macro_export]
macro_rules! println {
    ($($arg:tt)*) => {
        // no-op in release builds
    };
}
