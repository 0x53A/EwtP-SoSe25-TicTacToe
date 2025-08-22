#![no_std]
#![no_main]
#![feature(never_type)]

use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    dma_buffers,
    gpio::Io,
    i2s::master::{DataFormat, Standard},
    main,
    peripherals::Peripherals,
    time::Rate,
    uart::Uart,
};

use anyhow::{Result, anyhow};
use esp_println::println;
use heapless::Vec;

use esp_alloc as _;
use microfft::real::rfft_512;
use smart_leds::RGB8;
use smart_leds::SmartLedsWrite;

type NeopixelT<'a> = ws2812_spi::Ws2812<esp_hal::spi::master::Spi<'a, esp_hal::Blocking>>;

#[main]
fn main() -> ! {
    println!("Hello, world!");

    match _main() {
        Err(_e) => loop {},
    }
}

fn _main() -> Result<!> {
    esp_alloc::heap_allocator!(size: 72 * 1024);

    let peripherals: Peripherals = esp_hal::init(esp_hal::Config::default());
    let delay = Delay::new();

    // Setup SPI for NeoPixel
    let spi = esp_hal::spi::master::Spi::new(
        peripherals.SPI2,
        esp_hal::spi::master::Config::default().with_frequency(Rate::from_mhz(4)),
    )?
    .with_mosi(peripherals.GPIO48);

    let mut neopixel: NeopixelT = ws2812_spi::Ws2812::new(spi);

    let blue = smart_leds::colors::BLUE;
    neopixel.write([blue]).map_err(|err| anyhow!("{:?}", err))?;


    // todo

    // esp_hal::

    loop { }
}
