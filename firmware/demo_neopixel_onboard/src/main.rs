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

use embedded_hal::delay::DelayNs;

type NeopixelT<'a> = ws2812_spi::Ws2812<esp_hal::spi::master::Spi<'a, esp_hal::Blocking>>;


esp_bootloader_esp_idf::esp_app_desc!();


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
    let mut delay = Delay::new();
    let io = Io::new(peripherals.IO_MUX);

    // Setup SPI for NeoPixel
    let spi = esp_hal::spi::master::Spi::new(
        peripherals.SPI2,
        esp_hal::spi::master::Config::default().with_frequency(Rate::from_mhz(4)),
    )?
    .with_mosi(peripherals.GPIO48);

    let mut neopixel: NeopixelT = ws2812_spi::Ws2812::new(spi);

    // Setup UART
    let config = esp_hal::uart::Config::default().with_baudrate(115200);
    let mut uart = Uart::new(peripherals.UART1, config)?
        .with_rx(peripherals.GPIO17)
        .with_tx(peripherals.GPIO8);

    let blue = smart_leds::colors::BLUE;
    let green = smart_leds::colors::GREEN;
    let red = smart_leds::colors::RED;
    neopixel.write([blue]).map_err(|err| anyhow!("{:?}", err))?;

    loop {
        neopixel.write([green]).map_err(|err| anyhow!("{:?}", err))?;
        delay.delay_ms(100);
        neopixel.write([blue]).map_err(|err| anyhow!("{:?}", err))?;
        delay.delay_ms(100);
        neopixel.write([red]).map_err(|err| anyhow!("{:?}", err))?;
        delay.delay_ms(100);
    }
}


fn update_led(r: u8, g: u8, b: u8, neopixel: &mut NeopixelT) -> Result<()> {
    let color = RGB8::new(r, g, b);

    // Update neopixel
    neopixel
        .write([color])
        .map_err(|err| anyhow!("{:?}", err))?;

    Ok(())
}
