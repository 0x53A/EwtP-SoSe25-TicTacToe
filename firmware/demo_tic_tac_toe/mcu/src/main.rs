#![no_std]
#![no_main]
#![feature(never_type)]
#![feature(c_variadic)]

mod game;
mod game_rendering;
mod tinyusb_callbacks;

use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::Duration;
use esp_backtrace as _;
use esp_hal::{
    delay::Delay,
    dma::{DmaRxBuf, DmaTxBuf},
    dma_buffers,
    peripherals::Peripherals,
    system::{CpuControl, Stack},
    time::Rate,
    timer::{AnyTimer, timg::TimerGroup},
};

use anyhow::{Result, anyhow};
use esp_println::println;
use static_cell::StaticCell;

use esp_alloc as _;

use crate::{
    game::{GameStage, KeyboardInput},
    game_rendering::render_task,
};

extern crate alloc;

use alloc::{boxed::Box, format};
use core::ptr::addr_of_mut;

use smart_leds::RGB8;
use smart_leds::SmartLedsWrite;

macro_rules! error_with_location {
    ($msg:expr) => {
        anyhow!("{} at {}:{}", $msg, file!(), line!())
    };
    ($fmt:expr, $($arg:tt)*) => {
        anyhow!("{} at {}:{}", format!($fmt, $($arg)*), file!(), line!())
    };
}

const MATRIX_WIDTH: usize = 16; // 3x3 grid plus borders
const MATRIX_HEIGHT: usize = 16; // 3x3 grid plus borders
const MATRIX_LENGTH: usize = MATRIX_WIDTH * MATRIX_HEIGHT;
const TOTAL_NEOPIXEL_LENGTH: usize = MATRIX_LENGTH;

// Prerendered version of the NeoPixel driver for better performance
type NeopixelT<'a> = ws2812_spi::prerendered::Ws2812<
    'static,
    esp_hal::spi::master::SpiDmaBus<'a, esp_hal::Blocking>,
>;

static mut APP_CORE_STACK: Stack<8192> = Stack::new();

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

#[embassy_executor::task]
async fn neopixel_task(
    spi: esp_hal::spi::master::SpiDmaBus<'static, esp_hal::Blocking>,
    update_signal: &'static Signal<CriticalSectionRawMutex, Box<[RGB8]>>,
) -> ! {
    println!("Neopixel task started");

    let neopixel_buffer = Box::leak(Box::new([0u8; 12 * TOTAL_NEOPIXEL_LENGTH + 140]));
    let mut neopixel: NeopixelT = ws2812_spi::prerendered::Ws2812::new(spi, neopixel_buffer);

    // // demo
    // let mut delay = Delay::new();
    // for i in 0..256 {
    //     let mut colors = [RGB8::new(0, 0, 0); MATRIX_LENGTH];
    //     colors[i] = RGB8::new(255, 0, 0);
    //     if let Err(e) = neopixel.write(colors) {
    //         println!("Failed to write to NeoPixel: {:?}", e);
    //     }
    //     // delay.delay_ms(20);
    // }

    loop {
        let new_state = update_signal.wait().await;
        if let Err(e) = neopixel.write(new_state) {
            println!("Failed to write to NeoPixel: {:?}", e);
        }
    }
}

#[esp_hal_embassy::main]
async fn main(spawner: Spawner) -> ! {
    println!("Boot");

    match _main(spawner).await {
        Err(e) => {
            println!("Error: {:?}", e);
            loop {}
        }
        _ => loop {},
    }
}

async fn _main(spawner: Spawner) -> Result<!> {
    println!("Hello, world!");

    esp_alloc::heap_allocator!(size: 72 * 1024);

    // Initialize TicTacToe MATLAB code
    matlab_code::initialize();
    println!("TicTacToe MATLAB code initialized");

    let peripherals: Peripherals = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(peripherals.TIMG0);
    let timer0: AnyTimer = timg0.timer0.into();
    let timer1: AnyTimer = timg0.timer1.into();
    esp_hal_embassy::init([timer0, timer1]);

    // Enable and configure USB peripheral
    setup_usb_peripheral();

    let _delay = Delay::new();
    let mut cpu_control = CpuControl::new(peripherals.CPU_CTRL);

    // Setup SPI for NeoPixel
    println!("Setting up NeoPixel...");

    // Set up DMA for SPI
    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(1, 4 * 1024);
    let dma_rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer)
        .map_err(|err| error_with_location!("Failed to create DMA RX buffer: {:?}", err))?;
    let dma_tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer)
        .map_err(|err| error_with_location!("Failed to create DMA TX buffer: {:?}", err))?;

    let spi: esp_hal::spi::master::SpiDmaBus<'_, esp_hal::Blocking> =
        esp_hal::spi::master::Spi::new(
            peripherals.SPI2,
            esp_hal::spi::master::Config::default().with_frequency(Rate::from_khz(4_500)),
        )?
        .with_mosi(peripherals.GPIO21)
        .with_dma(peripherals.DMA_CH1)
        .with_buffers(dma_rx_buf, dma_tx_buf);

    static NEOPIXEL_SIGNAL: StaticCell<Signal<CriticalSectionRawMutex, Box<[RGB8]>>> =
        StaticCell::new();
    let neopixel_signal = &*NEOPIXEL_SIGNAL.init(Signal::new());

    // Start the second core with the NeoPixel task
    let _guard = cpu_control
        .start_app_core(unsafe { &mut *addr_of_mut!(APP_CORE_STACK) }, move || {
            static EXECUTOR: StaticCell<esp_hal_embassy::Executor> = StaticCell::new();
            let executor = EXECUTOR.init(esp_hal_embassy::Executor::new());
            executor.run(|spawner| {
                spawner.spawn(neopixel_task(spi, neopixel_signal)).ok();
            });
        })
        .unwrap();

    static GAMESTAGE_SIGNAL: StaticCell<Signal<CriticalSectionRawMutex, GameStage>> =
        StaticCell::new();
    let gamestage_signal = &*GAMESTAGE_SIGNAL.init(Signal::new());

    // spawn the rendering task
    println!("Spawning rendering task...");
    let spawn_result = spawner.spawn(render_task(gamestage_signal, neopixel_signal));
    if let Err(e) = spawn_result {
        println!("Failed to spawn render_task: {:?}", e);
    }

    println!("Spawning interrupt count task...");
    let spawn_result = spawner.spawn(print_interrupt_count_task());
    if let Err(e) = spawn_result {
        println!("Failed to spawn print_interrupt_count_task: {:?}", e);
    }
    println!("Spawned interrupt count task");

    static KEYBOARD_INPUT_SIGNAL: StaticCell<Signal<CriticalSectionRawMutex, KeyboardInput>> =
        StaticCell::new();
    let keyboard_input_signal: &Signal<CriticalSectionRawMutex, KeyboardInput> =
        &*KEYBOARD_INPUT_SIGNAL.init(Signal::new());

    static mut KEYBOARD_INPUT_SIGNAL_REF: Option<&Signal<CriticalSectionRawMutex, KeyboardInput>> =
        None;
    unsafe {
        KEYBOARD_INPUT_SIGNAL_REF = Some(keyboard_input_signal);
    }

    println!("Spawning game logic task...");
    let spawn_result = spawner.spawn(game::game_loop(keyboard_input_signal, gamestage_signal));
    if let Err(e) = spawn_result {
        println!("Failed to spawn game_logic_task: {:?}", e);
    }
    println!("Spawned game logic task");

    // This callback will be invoked for each HID report received
    tinyusb_callbacks::set_rust_hid_report_callback(Some(|dev_addr, instance, report, len| {
        println!(
            "HID report received: dev_addr={}, instance={}, len={}",
            dev_addr, instance, len
        );

        // Check if it's a keyboard report
        let proto = unsafe { tinyusb_sys::tuh_hid_interface_protocol(dev_addr, instance) };
        if proto == tinyusb_sys::hid_interface_protocol_enum_t::HID_ITF_PROTOCOL_KEYBOARD as u8
            && len as usize >= core::mem::size_of::<tinyusb_sys::hid_keyboard_report_t>()
        {
            let report = unsafe { &*(report as *const tinyusb_sys::hid_keyboard_report_t) };
            if let Some(key) = process_keyboard_input(report) {
                unsafe { KEYBOARD_INPUT_SIGNAL_REF.unwrap().signal(key) };
            }
        }
    }));

    // Initialize tinyusb host stack
    println!("Initializing TinyUSB...");
    init_tinyusb();
    println!("TinyUSB initialized");

    // Main loop, run both tinyusb and our game tasks
    loop {
        unsafe {
            tinyusb_sys::tuh_task_ext(u32::MAX, false);
        }

        embassy_futures::yield_now().await;
    }
}

fn process_keyboard_input(report: &tinyusb_sys::hid_keyboard_report_t) -> Option<KeyboardInput> {
    for &keycode in &report.keycode {
        if keycode == 0 {
            // No key pressed in this slot
            continue;
        }
        println!("Keycode: {:#X}", keycode);
    }

    // Check each keycode in the report
    for &keycode in &report.keycode {
        // Map keycodes to actual keys
        let key = match keycode {
            // Numpad keys for positions 1-9
            0x59 => KeyboardInput::Numpad(1), // Numpad 1
            0x5A => KeyboardInput::Numpad(2), // Numpad 2
            0x5B => KeyboardInput::Numpad(3), // Numpad 3
            0x5C => KeyboardInput::Numpad(4), // Numpad 4
            0x5D => KeyboardInput::Numpad(5), // Numpad 5
            0x5E => KeyboardInput::Numpad(6), // Numpad 6
            0x5F => KeyboardInput::Numpad(7), // Numpad 7
            0x60 => KeyboardInput::Numpad(8), // Numpad 8
            0x61 => KeyboardInput::Numpad(9), // Numpad 9

            // Number keys for positions 1-9
            0x1E => KeyboardInput::Number(1), // 1
            0x1F => KeyboardInput::Number(2), // 2
            0x20 => KeyboardInput::Number(3), // 3
            0x21 => KeyboardInput::Number(4), // 4
            0x22 => KeyboardInput::Number(5), // 5
            0x23 => KeyboardInput::Number(6), // 6
            0x24 => KeyboardInput::Number(7), // 7
            0x25 => KeyboardInput::Number(8), // 8
            0x26 => KeyboardInput::Number(9), // 9

            0x52 => KeyboardInput::ArrowUp,      // Up Arrow
            0x51 => KeyboardInput::ArrowDown,    // Down Arrow
            0x50 => KeyboardInput::ArrowLeft,    // Left Arrow
            0x4F => KeyboardInput::ArrowRight,   // Right Arrow
            0x58 | 0x28 => KeyboardInput::Enter, // Enter key

            _ => continue, // Ignore other keys
        };

        println!("Key pressed: {:?}", key);

        return Some(key);
    }
    None
}

// Set up the USB peripheral
fn setup_usb_peripheral() {
    unsafe {
        let system = esp32s3::SYSTEM::steal();
        // enable clock
        system
            .perip_clk_en0()
            .modify(|_, w| w.usb_clk_en().set_bit());

        // Reset USB peripheral by pulsing the reset register
        system.perip_rst_en0().modify(|_, w| w.usb_rst().set_bit());
        system
            .perip_rst_en0()
            .modify(|_, w| w.usb_rst().clear_bit());

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

        // drive strength
        let iomux = esp32s3::IO_MUX::steal();
        iomux.gpio(19).modify(|_, w| w.fun_drv().bits(3));
        iomux.gpio(20).modify(|_, w| w.fun_drv().bits(3));

        // override pull ups/downs
        // see esp-idf\components\esp_hw_support\usb_phy\usb_phy.c:160
        usb_wrap.otg_conf().modify(|_, w| {
            w.dp_pullup().clear_bit();
            w.dm_pullup().clear_bit();
            w.dp_pulldown().set_bit();
            w.dm_pulldown().set_bit();
            w.pad_pull_override().set_bit();

            w
        });

        // this is all copied from esp IDF usb initialization
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
