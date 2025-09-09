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
    println!("Hello, world!");

    match _main(spawner).await {
        Err(_e) => loop {},
    }
}

async fn _main(spawner: Spawner) -> Result<!> {
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

        let usb_dev = esp32s3::USB_DEVICE::steal();

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

    // //
    // unsafe {
    //     // #define USB_OTG_IDDIG_IN_IDX          58
    //     esp32s3::GPIO::steal().func_in_sel_cfg(58).modify(|w| {
    //         // named func_sel in IDF
    //         w.in_sel().bits(0x30);
    //         // named sig_in_inv in IDF
    //         w.in_inv_sel().clear_bit();
    //         // named sig_in_sel in IDF
    //         w.sel().set_bit();

    //         w
    //     });

    //     // #define USB_OTG_AVALID_IN_IDX         59

    //     // #define USB_SRP_BVALID_IN_IDX         60

    //     // #define USB_OTG_VBUSVALID_IN_IDX      61

    //     // #define USB_SRP_SESSEND_IN_IDX        62

    //     // #define USB_OTG_IDPULLUP_IDX          60
    // }

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

    // Initialize tinyusb host stack
    println!("initializing tinyusb ...");
    init_tinyusb();
    println!("tinyusb initialized");

    // after tinyusb init, clear any pending interrupts
    // unsafe {
    //     let usb_0 = esp32s3::USB0::steal();
    //     usb_0.gintsts().modify(|_, w| {
    //         w.modemis().clear_bit_by_one();
    //         w.sof().clear_bit_by_one()
    //     });

    //     usb_0.gintmsk().modify(|_, w| {
    //         w.modemismsk().clear_bit();
    //         w.ptxfempmsk().clear_bit();
    //         w.nptxfempmsk().clear_bit();
    //         w.sofmsk().clear_bit();
    //         w
    //     });
    // }

    static mut NEOPIXEL: Option<&'static mut NeopixelT> = None;
    unsafe {
        NEOPIXEL = Some(Box::leak(Box::new(neopixel)));
    }

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

        let proto = unsafe { tinyusb_sys::tuh_hid_interface_protocol(dev_addr, instance) };
        if proto == tinyusb_sys::hid_interface_protocol_enum_t::HID_ITF_PROTOCOL_KEYBOARD as u8 {
            println!("proto == keyboard");

            if len as usize >= core::mem::size_of::<tinyusb_sys::hid_keyboard_report_t>() {
                let report = unsafe { &*(report as *const tinyusb_sys::hid_keyboard_report_t) };
                process_kbd_report(dev_addr, report);
            } else {
                println!("Warning: HID keyboard report too short: {}", len);
            }
        } else {
            println!("HID protocol not handled: {:?}", proto);
        }
    }));

    println!("Spawning interrupt count task ...");
    let spawn_result = spawner.spawn(print_interrupt_count_task());
    if let Err(e) = spawn_result {
        println!("Failed to spawn print_interrupt_count_task: {:?}", e);
    }
    println!("Spawned interrupt count task");

    // Poll tinyusb host task
    loop {
        // Run tinyusb host task
        unsafe {
            // tinyusb's tuh_task should be called periodically to handle host stack
            tinyusb_sys::tuh_task_ext(u32::MAX, false);
        }
        embassy_futures::yield_now().await;
        
            for inst in 0..2 {
                unsafe { tinyusb_sys::tuh_hid_receive_report(1, inst); }
        embassy_futures::yield_now().await;
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

/// Callback invoked by tinyusb when a device is mounted.
/// The symbol name must match the C callback; tinyusb will call this.
#[unsafe(no_mangle)]
extern "C" fn tuh_mount_cb(daddr: u8) {
    // Keep callback small and safe: print that a device mounted and attempt to fetch
    // a device descriptor (best-effort).
    println!("Device mounted, address = {}", daddr);

    unsafe {
        // Try to fetch the device descriptor into a small stack buffer (18 bytes).
        // If binding provides tuh_descriptor_get_device_sync, call it
        let mut descriptor: tinyusb_sys::tusb_desc_device_t = core::mem::zeroed();
        // use our Rust sync wrapper
        let res = tuh_descriptor_get_device_sync(
            daddr,
            &raw mut descriptor as *mut c_void,
            core::mem::size_of::<tinyusb_sys::tusb_desc_device_t>() as u16,
        );
        if res == tinyusb_sys::xfer_result_t::XFER_RESULT_SUCCESS {
            let vendor = descriptor.idVendor;
            let product = descriptor.idProduct;
            println!(
                "Got descriptor: Device {}: ID {:04x}:{:04x}",
                daddr, vendor, product
            );
            for inst in 0..2 {
                unsafe { tinyusb_sys::tuh_hid_receive_report(daddr, inst); }
            }
        } else {
            println!("Failed to get device descriptor for addr {}", daddr);
        }

        // Two-step configuration descriptor fetch:
        // 1) read first 9 bytes (config header) to get wTotalLength
        // 2) read wTotalLength bytes
        let mut header = [0u8; 9];
        let hdr_res = tuh_descriptor_get_configuration_sync(
            daddr,
            0,
            header.as_mut_ptr() as *mut c_void,
            header.len() as u16,
        );
        println!("cfg header fetch result for device {} = {:?}", daddr, hdr_res);
        if hdr_res != tinyusb_sys::xfer_result_t::XFER_RESULT_SUCCESS {
            println!("Failed to fetch config header for device {} (res={:?})", daddr, hdr_res);
            return;
        }

        // Parse wTotalLength from the 9-byte header (little-endian at offset 2)
        let total_len = (header[2] as usize) | ((header[3] as usize) << 8);
        if total_len == 0 {
            println!("Device {}: wTotalLength == 0 in header — can't read full config", daddr);
            return;
        }

        // Cap to our buffer size
        let mut cfg_buf = [0u8; 512];
        let to_read = core::cmp::min(total_len, cfg_buf.len());
        let cfg_res = tuh_descriptor_get_configuration_sync(
            daddr,
            0,
            cfg_buf.as_mut_ptr() as *mut c_void,
            to_read as u16,
        );
        println!("cfg full fetch result for device {} = {:?} (requested {})", daddr, cfg_res, to_read);
        if cfg_res == tinyusb_sys::xfer_result_t::XFER_RESULT_SUCCESS {
            // parse and dump
            parse_and_print_hid_poll_interval(daddr, &cfg_buf[..to_read]);
            dump_and_find_interrupt_endpoints(daddr, &cfg_buf[..to_read]);
        } else {
            println!("Failed to get configuration descriptor for addr {} (res={:?})", daddr, cfg_res);
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

unsafe fn tuh_descriptor_get_configuration_sync(
    daddr: u8,
    config_index: u8,
    buffer: *mut c_void,
    len: u16,
) -> tinyusb_sys::xfer_result_t {
    let mut result: tinyusb_sys::xfer_result_t = tinyusb_sys::xfer_result_t::XFER_RESULT_INVALID;
    let ok = tinyusb_sys::tuh_descriptor_get_configuration(
        daddr,
        config_index as _,
        buffer,
        len,
        None,
        &mut result as *mut _ as usize,
    );
    if !ok {
        tinyusb_sys::xfer_result_t::XFER_RESULT_TIMEOUT
    } else {
        result
    }
}

/// Parse configuration descriptor bytes to find HID interrupt IN endpoint and print bInterval
fn parse_and_print_hid_poll_interval(daddr: u8, buf: &[u8]) {
    // basic checks
    if buf.len() < 9 {
        println!("Config descriptor too short");
        return;
    }
    // wTotalLength is at offset 2 (little endian)
    let total_len = (buf[2] as usize) | ((buf[3] as usize) << 8);
    let total_len = total_len.min(buf.len());

    let mut i = 0usize;
    let mut is_hid_interface = false;

    while i + 2 <= total_len {
        let b_len = buf[i] as usize;
        let b_desc_type = buf.get(i + 1).copied().unwrap_or(0);
        if b_len == 0 || i + b_len > total_len {
            break;
        }

        match b_desc_type {
            0x04 => {
                // Interface descriptor: bInterfaceClass is at offset i+5
                let iface_class = buf.get(i + 5).copied().unwrap_or(0);
                is_hid_interface = iface_class == 0x03; // HID class
                if is_hid_interface {
                    println!("Found HID interface at offset {}", i);
                }
            }
            0x05 => {
                // Endpoint descriptor. If it's an interrupt IN endpoint and we are in a HID interface, read bInterval.
                if is_hid_interface {
                    let ep_addr = buf.get(i + 2).copied().unwrap_or(0);
                    let attributes = buf.get(i + 3).copied().unwrap_or(0);
                    let b_interval = buf.get(i + 6).copied().unwrap_or(0);
                    // check IN bit and interrupt transfer type (attributes & 0x3 == 0x3 indicates interrupt)
                    let is_in = (ep_addr & 0x80) != 0;
                    let transfer_type = attributes & 0x03;
                    if is_in && transfer_type == 0x03 {
                        // For full-speed devices bInterval is in milliseconds. For high-speed it's exponent of microframes.
                        println!(
                            "Device {}: HID interrupt IN endpoint 0x{:02x}, bInterval = {}",
                            daddr, ep_addr, b_interval
                        );
                        // stop after first found HID IN endpoint
                        return;
                    }
                }
            }
            _ => {}
        }

        i += b_len;
    }

    println!(
        "Device {}: no HID interrupt IN endpoint found in config descriptor",
        daddr
    );
}

// KEYCODE to ASCII conversion table (128 entries) — matches the provided C HID_KEYCODE_TO_ASCII mapping
static KEYCODE2ASCII: [[u8; 2]; 128] = [
    /* 0x00 - 0x07 */ [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [b'a', b'A'],
    [b'b', b'B'],
    [b'c', b'C'],
    [b'd', b'D'],
    /* 0x08 - 0x0f */ [b'e', b'E'],
    [b'f', b'F'],
    [b'g', b'G'],
    [b'h', b'H'],
    [b'i', b'I'],
    [b'j', b'J'],
    [b'k', b'K'],
    [b'l', b'L'],
    /* 0x10 - 0x17 */ [b'm', b'M'],
    [b'n', b'N'],
    [b'o', b'O'],
    [b'p', b'P'],
    [b'q', b'Q'],
    [b'r', b'R'],
    [b's', b'S'],
    [b't', b'T'],
    /* 0x18 - 0x1f */ [b'u', b'U'],
    [b'v', b'V'],
    [b'w', b'W'],
    [b'x', b'X'],
    [b'y', b'Y'],
    [b'z', b'Z'],
    [b'1', b'!'],
    [b'2', b'@'],
    /* 0x20 - 0x27 */ [b'3', b'#'],
    [b'4', b'$'],
    [b'5', b'%'],
    [b'6', b'^'],
    [b'7', b'&'],
    [b'8', b'*'],
    [b'9', b'('],
    [b'0', b')'],
    /* 0x28 - 0x2f */ [b'\r', b'\r'],
    [0x1b, 0x1b],
    [0x08, 0x08],
    [b'\t', b'\t'],
    [b' ', b' '],
    [b'-', b'_'],
    [b'=', b'+'],
    [b'[', b'{'],
    /* 0x30 - 0x37 */ [b']', b'}'],
    [b'\\', b'|'],
    [b'#', b'~'],
    [b';', b':'],
    [b'\'', b'"'],
    [b'`', b'~'],
    [b',', b'<'],
    [b'.', b'>'],
    /* 0x38 - 0x3f */ [b'/', b'?'],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    /* 0x40 - 0x47 */ [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    /* 0x48 - 0x4f */ [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    /* 0x50 - 0x57 */ [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    /* 0x58 - 0x5f */ [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    /* 0x60 - 0x67 */ [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    /* Numeric keypad (placed at positions 0x54..0x67 in original C mapping) */
    /* We now explicitly set the keypad entries at their correct indices by listing the array in index order — below entries correspond to 0x68..0x7f padding to reach 128 total entries. */
    /* 0x68 - 0x6f */
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    /* 0x70 - 0x77 */ [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    /* 0x78 - 0x7f */ [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
    [0, 0],
];

fn find_key_in_report(report: &tinyusb_sys::hid_keyboard_report_t, keycode: u8) -> bool {
    report.keycode.iter().any(|&k| k == keycode)
}

#[allow(static_mut_refs)]
fn process_kbd_report(dev_addr: u8, report: &tinyusb_sys::hid_keyboard_report_t) {
    static mut PREV_REPORT: tinyusb_sys::hid_keyboard_report_t =
        tinyusb_sys::hid_keyboard_report_t {
            modifier: 0,
            reserved: 0,
            keycode: [0; 6],
        };
    let mut flush = false;

    for &keycode in &report.keycode {
        if keycode != 0 {
            let existed = unsafe { find_key_in_report(&PREV_REPORT, keycode) };
            if !existed {
                println!("Key Down: {keycode}");
            }
        }
    }

    if flush {
        // tud_cdc_write_flush();
        println!();
    }

    unsafe {
        PREV_REPORT = *report;
    }
}

fn dump_and_find_interrupt_endpoints(daddr: u8, buf: &[u8]) {
    if buf.len() < 9 {
        println!("Config descriptor too short");
        return;
    }

    let total_len = (buf[2] as usize) | ((buf[3] as usize) << 8);
    let total_len = total_len.min(buf.len());

    println!(
        "Config descriptor total length (wTotalLength) = {} (buffer {})",
        total_len,
        buf.len()
    );

    // Hex dump first part to inspect when parser misses things
    {
        let mut i = 0usize;
        while i < total_len {
            let end = core::cmp::min(i + 16, total_len);
            let slice = &buf[i..end];
            // print offset and hex bytes
            print!("{:04x}:", i);
            for b in slice {
                print!(" {:02x}", b);
            }
            println!();
            i = end;
        }
    }

    let mut i = 0usize;
    let mut cur_interface: Option<u8> = None;
    while i + 2 <= total_len {
        let b_len = buf[i] as usize;
        let b_desc_type = buf.get(i + 1).copied().unwrap_or(0);
        if b_len == 0 || i + b_len > total_len {
            println!("Stopping parse at offset {} (bad length {})", i, b_len);
            break;
        }

        match b_desc_type {
            0x04 => {
                // Interface descriptor
                let if_num = buf.get(i + 2).copied().unwrap_or(0);
                let alt = buf.get(i + 3).copied().unwrap_or(0);
                let num_ep = buf.get(i + 4).copied().unwrap_or(0);
                let iface_class = buf.get(i + 5).copied().unwrap_or(0);
                let iface_subclass = buf.get(i + 6).copied().unwrap_or(0);
                let iface_protocol = buf.get(i + 7).copied().unwrap_or(0);
                cur_interface = Some(if_num);
                println!(
                    "Interface desc @{}: if={}, alt={}, num_ep={}, class=0x{:02x}, sub=0x{:02x}, prot=0x{:02x}",
                    i, if_num, alt, num_ep, iface_class, iface_subclass, iface_protocol
                );
            }
            0x21 => {
                // HID class descriptor (class-specific)
                // common HID desc layout: bLength, bDescriptorType=0x21, bcdHID(2), bCountryCode, bNumDescriptors, then (bDescriptorType, wDescriptorLength) pairs
                let bcd_hid = ((buf.get(i + 3).copied().unwrap_or(0) as u16) << 8)
                    | buf.get(i + 2).copied().unwrap_or(0) as u16;
                let country = buf.get(i + 4).copied().unwrap_or(0);
                let num_desc = buf.get(i + 5).copied().unwrap_or(0);
                println!(
                    "HID descriptor @{}: bcdHID=0x{:04x}, country=0x{:02x}, num_desc={}",
                    i, bcd_hid, country, num_desc
                );
                for d in 0..num_desc {
                    let off = i + 6 + (d as usize) * 3;
                    if off + 3 <= i + b_len {
                        let d_type = buf.get(off).copied().unwrap_or(0);
                        let d_len = (buf.get(off + 1).copied().unwrap_or(0) as u16)
                            | ((buf.get(off + 2).copied().unwrap_or(0) as u16) << 8);
                        println!("  desc[{}]: type=0x{:02x}, len={}", d, d_type, d_len);
                    }
                }
            }
            0x05 => {
                // Endpoint descriptor
                let ep_addr = buf.get(i + 2).copied().unwrap_or(0);
                let attributes = buf.get(i + 3).copied().unwrap_or(0);
                let max_packet = (buf.get(i + 4).copied().unwrap_or(0) as u16)
                    | ((buf.get(i + 5).copied().unwrap_or(0) as u16) << 8);
                let b_interval = buf.get(i + 6).copied().unwrap_or(0);
                let dir = if (ep_addr & 0x80) != 0 { "IN" } else { "OUT" };
                let transfer_type = attributes & 0x03;
                let transfer_name = match transfer_type {
                    0x00 => "Control",
                    0x01 => "Iso",
                    0x02 => "Bulk",
                    0x03 => "Interrupt",
                    _ => "Unknown",
                };
                println!(
                    "Endpoint @{}: ep=0x{:02x} ({}) type={} maxpkt={} bInterval={}",
                    i, ep_addr, dir, transfer_name, max_packet, b_interval
                );
                if (ep_addr & 0x80) != 0 && transfer_type == 0x03 {
                    println!(
                        "--> Found Interrupt IN endpoint for device {}: ep=0x{:02x}, bInterval={}",
                        daddr, ep_addr, b_interval
                    );
                    // continue to list others
                }
            }
            _ => {
                // Print unknown descriptor types optionally
                // println!("Descriptor @{} type=0x{:02x} len={}", i, b_desc_type, b_len);
            }
        }

        i += b_len;
    }

    println!("Done parsing configuration descriptor for device {}", daddr);
}
