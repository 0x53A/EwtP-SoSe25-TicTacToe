#![no_std]
#![no_main]
#![feature(never_type)]
#![feature(c_variadic)]

mod tinyusb_callbacks;

use core::any::Any;

use embassy_executor::Spawner;
use embassy_time::Duration;
use embedded_hal::delay::DelayNs;
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
use esp_println::{print, println};
use heapless::Vec;

use esp_alloc as _;

extern crate alloc;

use alloc::boxed::Box;
use microfft::real::rfft_512;
use smart_leds::RGB8;
use smart_leds::SmartLedsWrite;

type NeopixelT<'a> = ws2812_spi::Ws2812<esp_hal::spi::master::Spi<'a, esp_hal::Blocking>>;

use core::ffi::c_void;
use core::ptr;

esp_bootloader_esp_idf::esp_app_desc!();

#[embassy_executor::task]
pub async fn print_interrupt_count_task() {
    println!("Starting interrupt count task");
    let mut ticker = embassy_time::Ticker::every(Duration::from_secs(1));
    let mut prev_count = 0;
    loop {
        let count = tinyusb_callbacks::INTERRUPT_COUNTER.load(core::sync::atomic::Ordering::SeqCst);
        println!("Interrupts: {}/1s", count - prev_count);
        prev_count = count;

        ticker.next().await;
    }
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    println!("Boot");

    match _main(spawner).await {
        Err(_e) => loop {},
    }
}

async fn _main(spawner: Spawner) -> Result<!> {
    println!("Hello, world!");

    esp_alloc::heap_allocator!(size: 72 * 1024);

    let peripherals: Peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let timer0: AnyTimer = timg0.timer0.into();
    let timer1: AnyTimer = timg0.timer1.into();
    esp_hal_embassy::init([timer0, timer1]);

    // enable and configure USB peripheral, before initializing tinyusb
    unsafe {
        let system = esp32s3::SYSTEM::steal();
        // enable clock
        system
            .perip_clk_en0()
            .modify(|_, w| w.usb_clk_en().set_bit());

        // Reset USB peripheral by pulsing the reset register
        system.perip_rst_en0().modify(|_, w| w.usb_rst().set_bit());
        system.perip_rst_en0().modify(|_, w| w.usb_rst().clear_bit());

        let usb_dev = esp32s3::USB_DEVICE::steal();

        // clear pending interrupts once
        usb_dev
            .int_clr()
            .write(|w| w.serial_in_empty().clear_bit_by_one());

        let usb_0 = esp32s3::USB0::steal();
        usb_0
            .gintsts()
            .modify(|_, w| w.modemis().clear_bit_by_one().sof().clear_bit_by_one());


        let usb_wrap = esp32s3::USB_WRAP::steal();
        usb_wrap.otg_conf().modify(|_, w| {
            w.usb_pad_enable().set_bit();
            w.phy_sel().clear_bit();
            w.clk_en().set_bit();
            w.ahb_clk_force_on().set_bit();
            w.phy_clk_force_on().set_bit();

            // override VBUS sensing
            w.srp_sessend_override().set_bit();
            w.srp_sessend_value().clear_bit();

            w
        });

        let rtc_cntl = esp32s3::RTC_CNTL::steal();
        rtc_cntl.usb_conf().modify(|_, w| {
            w.sw_hw_usb_phy_sel().set_bit();
            w.sw_usb_phy_sel().set_bit()
        });
    }

    // drive strength
    unsafe {
        let iomux = esp32s3::IO_MUX::steal();
        iomux.gpio(19).modify(|_, w| w.fun_drv().bits(3));
        iomux.gpio(20).modify(|_, w| w.fun_drv().bits(3));
    }

    // override pull ups/downs
    // see esp-idf\components\esp_hw_support\usb_phy\usb_phy.c:160
    unsafe {
        let usb_wrap = esp32s3::USB_WRAP::steal();
        usb_wrap.otg_conf().modify(|_, w| {
            w.dp_pullup().clear_bit();
            w.dm_pullup().clear_bit();
            w.dp_pulldown().set_bit();
            w.dm_pulldown().set_bit();
            w.pad_pull_override().set_bit();

            w
        });
    }

    // this is all copied from esp IDF usb initialization
    unsafe {
        // map OTG input signals to constant-0 / constant-1 GPIO-matrix sources
        const GPIO_MATRIX_CONST_ZERO_INPUT: u8 = 0x30;
        const GPIO_MATRIX_CONST_ONE_INPUT: u8 = 0x38;

        let gpio = esp32s3::GPIO::steal();

        // USB_OTG_IDDIG_IN_IDX = 58  -> connect to const 0 (connected connector is A side)
        gpio.func_in_sel_cfg(58).modify(|_, w| {
            w.in_sel().bits(GPIO_MATRIX_CONST_ZERO_INPUT);
            w.in_inv_sel().clear_bit();
            w.sel().set_bit();
            w
        });

        // USB_SRP_BVALID_IN_IDX = 60 -> connect to const 0
        gpio.func_in_sel_cfg(60).modify(|_, w| {
            w.in_sel().bits(GPIO_MATRIX_CONST_ZERO_INPUT);
            w.in_inv_sel().clear_bit();
            w.sel().set_bit();
            w
        });

        // USB_OTG_VBUSVALID_IN_IDX = 61 -> connect to const 1 (we are receiving valid VBUS from host)
        gpio.func_in_sel_cfg(61).modify(|_, w| {
            w.in_sel().bits(GPIO_MATRIX_CONST_ONE_INPUT);
            w.in_inv_sel().clear_bit();
            w.sel().set_bit();
            w
        });

        // USB_OTG_AVALID_IN_IDX = 59 -> connect to const 1 (force USB host mode)
        gpio.func_in_sel_cfg(59).modify(|_, w| {
            w.in_sel().bits(GPIO_MATRIX_CONST_ONE_INPUT);
            w.in_inv_sel().clear_bit();
            w.sel().set_bit();
            w
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

    for _ in 0..5 {
        neopixel.write([red]).map_err(|err| anyhow!("{:?}", err))?;
        delay.delay_ms(100u32);
        neopixel.write([blue]).map_err(|err| anyhow!("{:?}", err))?;
        delay.delay_ms(100u32);
    }
    

    static mut NEOPIXEL: Option<&'static mut NeopixelT> = None;
    unsafe {
        NEOPIXEL = Some(Box::leak(Box::new(neopixel)));

    // this callback will be invoked for each HID report received
    tinyusb_callbacks::set_rust_hid_report_callback(Some(|dev_addr, instance, report, len| {
        println!(
            "HID report received: dev_addr={}, instance={}, report={:?}, len={}",
            dev_addr, instance, report, len
        );

        #[allow(static_mut_refs)]
        let neopixel = unsafe { NEOPIXEL.as_mut().unwrap() };

        static mut I: u32 = 0;
        let i = unsafe {
            I += 1;
            I
        };

        neopixel
            .write([if i % 2 == 0 {
                smart_leds::colors::GREEN
            } else {
                smart_leds::colors::RED
            }])
            .unwrap();
    }));

    // Initialize tinyusb host stack
    println!("initializing tinyusb ...");
    init_tinyusb();
    println!("tinyusb initialized");
    }


    println!("Spawning interrupt count task ...");
    let spawn_result = spawner.spawn(print_interrupt_count_task());
    if let Err(e) = spawn_result {
        println!("Failed to spawn print_interrupt_count_task: {:?}", e);
    }
    println!("Spawned interrupt count task");

    // main loop, run both tinyusb and our own tasks
    loop {
        unsafe {
            tinyusb_sys::tuh_task_ext(u32::MAX, false);
        }
        embassy_futures::yield_now().await;
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

        rh_init.speed = tinyusb_sys::tusb_speed_t::TUSB_SPEED_FULL as _;

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


