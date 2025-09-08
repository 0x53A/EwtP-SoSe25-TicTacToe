#![no_std]
#![no_main]
#![feature(never_type)]
#![feature(c_variadic)]

mod tinyusb_callbacks;

use core::any::Any;

use embassy_executor::Spawner;
use embedded_hal::delay::DelayNs as _;
use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    dma_buffers,
    gpio::Io,
    i2s::master::{DataFormat, Standard},
    main,
    peripherals::Peripherals,
    time::Rate,
    timer::{AnyTimer, timg::TimerGroup},
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

use core::ffi::c_void;
use core::ptr;

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    println!("Hello, world!");

    match _main() {
        Err(_e) => loop {},
    }
}

fn _main() -> Result<!> {
    println!("Boot 2");

    esp_alloc::heap_allocator!(size: 72 * 1024);

    let peripherals: Peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let timer0: AnyTimer = timg0.timer0.into();
    let timer1: AnyTimer = timg0.timer1.into();
    esp_hal_embassy::init([timer0, timer1]);

    unsafe {
        let system = esp32s3::SYSTEM::steal();
        // enable clock
        system
            .perip_clk_en0()
            .modify(|_, w| w.usb_clk_en().set_bit());

        // Reset USB peripheral
        system.perip_rst_en0().modify(|_, w| w.usb_rst().set_bit());
        system
            .perip_rst_en0()
            .modify(|_, w| w.usb_rst().clear_bit());

        let usb_wrap = esp32s3::USB_WRAP::steal();
        usb_wrap.otg_conf().modify(|_, w| {
            w.usb_pad_enable().set_bit();
            w.phy_sel().clear_bit();
            w.clk_en().set_bit();
            w.ahb_clk_force_on().set_bit();
            w.phy_clk_force_on().set_bit();

            // override VBUS sensing
            w.srp_sessend_override().set_bit();
            w.srp_sessend_value().clear_bit()
        });

        let rtc_cntl = esp32s3::RTC_CNTL::steal();
        rtc_cntl.usb_conf().modify(|_, w| {
            w.sw_hw_usb_phy_sel().set_bit();
            w.sw_usb_phy_sel().set_bit()
        });
    }

    let mut delay = Delay::new();

    // Setup SPI for NeoPixel
    let spi = esp_hal::spi::master::Spi::new(
        peripherals.SPI2,
        esp_hal::spi::master::Config::default().with_frequency(Rate::from_mhz(4)),
    )?
    .with_mosi(peripherals.GPIO48);

    let mut neopixel: NeopixelT = ws2812_spi::Ws2812::new(spi);

    let blue = smart_leds::colors::BLUE;
    let red = smart_leds::colors::RED;
    neopixel.write([red]).map_err(|err| anyhow!("{:?}", err))?;

    for _ in 0..10 {
        neopixel.write([red]).map_err(|err| anyhow!("{:?}", err))?;
        delay.delay_ms(100u32);
        neopixel.write([blue]).map_err(|err| anyhow!("{:?}", err))?;
        delay.delay_ms(100u32);
    }

    // Initialize tinyusb host stack
    println!("initializing tinyusb ...");
    init_tinyusb();
    println!("tinyusb initialized");

    // Poll tinyusb host task and do a simple blink loop
    loop {
        // Run tinyusb host task
        unsafe {
            // tinyusb's tuh_task should be called periodically to handle host stack
            tinyusb_sys::tuh_task_ext(u32::MAX, false);
        }
    }
}

/// Initialize tinyusb as a host on root port 0
fn init_tinyusb() {
    // Build the C struct for initialization. Field names/types come from tinyusb_sys bindings.
    // Use unsafe since we call into C.
    unsafe {
        // Prepare a default-initialized struct and set fields we need.
        let mut rh_init: tinyusb_sys::tusb_rhport_init_t = core::mem::zeroed();

        // role = HOST
        rh_init.role = tinyusb_sys::tusb_role_t::TUSB_ROLE_HOST as _;

        rh_init.speed = tinyusb_sys::tusb_speed_t::TUSB_SPEED_LOW as _;

        // Initialize tinyusb host on root port 0 through the rhport initializer
        let ok = tinyusb_sys::tuh_rhport_init(
            0u8 as _,
            &rh_init as *const tinyusb_sys::tusb_rhport_init_t,
        );
        if !ok {
            println!("tuh_rhport_init failed");
        }
    }

    println!("tinyusb host initialized");
}

/// Callback invoked by tinyusb when a device is mounted.
/// The symbol name must match the C callback; tinyusb will call this.
#[unsafe(no_mangle)]
extern "C" fn tuh_mount_cb(daddr: u8) {
    // Keep callback small and safe: print that a device mounted and attempt to fetch
    // a device descriptor (best-effort).
    println!("Device mounted, address = {}", daddr);

    unsafe {
        // Try to fetch the device descriptor into a small stack buffer (18 bytes).
        // If binding provides tuh_descriptor_get_device_sync, call it; otherwise ignore errors.
        let mut dev_buf: [u8; 18] = [0u8; 18];
        // use our Rust sync wrapper
        let res = tuh_descriptor_get_device_sync(daddr, dev_buf.as_mut_ptr() as *mut c_void, 18);
        if res == tinyusb_sys::xfer_result_t::XFER_RESULT_SUCCESS {
            // Extract some fields from descriptor: idVendor (offset 8..10), idProduct (10..12)
            let id_vendor = u16::from_le_bytes([dev_buf[8], dev_buf[9]]);
            let id_product = u16::from_le_bytes([dev_buf[10], dev_buf[11]]);
            println!("Device {}: ID {:04x}:{:04x}", daddr, id_vendor, id_product);
            tinyusb_sys::tuh_hid_receive_report(daddr, 0);
        } else {
            println!("Failed to get device descriptor for addr {}", daddr);
        }
    }
}

/// Callback invoked by tinyusb when a device is unmounted.
#[unsafe(no_mangle)]
extern "C" fn tuh_umount_cb(daddr: u8) {
    println!("Device removed, address = {}", daddr);
}

/// Sync version of tuh_descriptor_get_device()
///
/// Implements the TU_API_SYNC behavior from the tinyusb C headers:
/// - allocate a local result variable
/// - call async API with NULL callback and &result as uintptr
/// - if async API returns false -> return XFER_RESULT_TIMEOUT
/// - otherwise return the result value written by the stack
unsafe fn tuh_descriptor_get_device_sync(
    daddr: u8,
    buffer: *mut c_void,
    len: u16,
) -> tinyusb_sys::xfer_result_t {
    let mut result: tinyusb_sys::xfer_result_t = tinyusb_sys::xfer_result_t::XFER_RESULT_INVALID;
    let ok = unsafe {
        tinyusb_sys::tuh_descriptor_get_device(
            daddr,
            buffer,
            len,
            None,                           // NULL callback
            &mut result as *mut _ as usize, // (uintptr_t) &result
        )
    };
    if !ok {
        tinyusb_sys::xfer_result_t::XFER_RESULT_TIMEOUT
    } else {
        result
    }
}
