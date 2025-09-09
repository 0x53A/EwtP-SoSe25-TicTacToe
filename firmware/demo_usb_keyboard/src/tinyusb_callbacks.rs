use core::ffi::c_char;
use core::ffi::c_void;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use esp_hal::interrupt::CpuInterrupt;
use esp_hal::interrupt::Priority;
use esp_hal::system::Cpu;
use esp_println::{print, println};
use esp32s3::Interrupt;
use esp32s3::usb0::GINTSTS;
use esp32s3::usb0::gintsts::GINTSTS_SPEC;

// store the registered handler and arg so the trampoline can forward the interrupt
static TUSB_HANDLER: AtomicUsize = AtomicUsize::new(0);
static TUSB_HANDLER_ARG: AtomicUsize = AtomicUsize::new(0);

static TUSB_BOUND: AtomicBool = AtomicBool::new(false);

pub static INTERRUPT_COUNTER: AtomicU32 = AtomicU32::new(0);

unsafe fn dump_usb_mode_and_clear() {
    // steal PAC handles locally
    let usb0 = esp32s3::USB0::steal();
    let usb_wrap = esp32s3::USB_WRAP::steal();
    let usb_dev = esp32s3::USB_DEVICE::steal();

    // read typed fields where available
    let gint = usb0.gintsts().read();
    let gintmsk = usb0.gintmsk().read().bits();
    let gotg = usb0.gotgint().read().bits();
    let gahb = usb0.gahbcfg().read().bits();
    let gusb = usb0.gusbcfg().read().bits();

    let wrap_otg = usb_wrap.otg_conf().read().bits();
    let dev_int_raw = usb_dev.int_raw().read().bits();

    println!("[dump] GINTSTS bits={:#010x}", usb0.gintsts().read().bits());
    println!(
        "[dump] flags: CURMOD_INT={} MODEMIS={} SOF={} RXFLVL={}",
        gint.curmod_int().bit(),
        gint.modemis().bit(),
        gint.sof().bit(),
        gint.rxflvi().bit()
    );
    println!(
        "[dump] GINTMSK={:#010x} GOTGINT={:#010x} GAHBCFG={:#010x} GUSBCFG={:#010x}",
        gintmsk, gotg, gahb, gusb
    );
    println!(
        "[dump] WRAP_OTG={:#010x} DEV_INT_RAW={:#010x}",
        wrap_otg, dev_int_raw
    );

    // // conservative clears (debug only):
    // // - clear MODEMIS in GINTSTS (write-1-to-clear field)
    // if gint.modemis().bit() {
    //     // using field writer (write-1-to-clear)
    //     usb0.gintsts().write(|w| w.modemis().clear_bit_by_one());
    //     println!("[dump] cleared GINTSTS.MODEMIS");
    // }

    // // - clear wrapper SERIAL_IN_EMPTY (USB_DEVICE.INT_RAW bit 3) to stop wrapper-level NVIC if set
    // if (dev_int_raw & (1 << 3)) != 0 {
    //     usb_dev.int_clr().write(|w| w.serial_in_empty().clear_bit_by_one());
    //     println!("[dump] cleared USB_DEVICE.INT_RAW.SERIAL_IN_EMPTY");
    // }
}

pub unsafe extern "C" fn interrupt_trampoline() {
    INTERRUPT_COUNTER.fetch_add(1, Ordering::SeqCst);

    // snapshot relevant registers using PAC readers (typed accessors)
    let usb0 = esp32s3::USB0::steal();
    let pre_gint = usb0.gintsts().read();
    let pre_gintmsk = usb0.gintmsk().read().bits();
    let pre_gotgint = usb0.gotgint().read().bits();
    let pre_gahbcfg = usb0.gahbcfg().read().bits();
    let pre_gusbcfg = usb0.gusbcfg().read().bits();

    let usb_dev = esp32s3::USB_DEVICE::steal();
    let pre_dev_int_raw = usb_dev.int_raw().read().bits();
    let pre_dev_int_ena = usb_dev.int_ena().read().bits();
    let pre_dev_int_st = usb_dev.int_st().read().bits();

    let usb_wrap = esp32s3::USB_WRAP::steal();
    let pre_wrap_otg = usb_wrap.otg_conf().read().bits();

    // forward to saved handler
    let h = TUSB_HANDLER.load(Ordering::SeqCst);
    if h == 0 {
        return;
    }
    let arg = TUSB_HANDLER_ARG.load(Ordering::SeqCst) as *mut c_void;
    let handler: extern "C" fn(*mut c_void) = core::mem::transmute(h);
    handler(arg);

    // post snapshot using readers
    let post_gint = usb0.gintsts().read();
    let post_gintmsk = usb0.gintmsk().read().bits();
    let post_gotgint = usb0.gotgint().read().bits();
    let post_gahbcfg = usb0.gahbcfg().read().bits();
    let post_gusbcfg = usb0.gusbcfg().read().bits();

    let post_dev_int_raw = usb_dev.int_raw().read().bits();
    let post_dev_int_ena = usb_dev.int_ena().read().bits();
    let post_dev_int_st = usb_dev.int_st().read().bits();

    let post_wrap_otg = usb_wrap.otg_conf().read().bits();

    // print only if any named GINTSTS field was set before and remains set after
    macro_rules! check_gint {
        ($name:expr, $field:ident) => {
            if pre_gint.$field().bit() && post_gint.$field().bit() {
                println!("[irq] USB0.GINTSTS {}", $name);
            }
        };
    }

    let mut any_remaining = false;

    // // check commonly interesting fields (names come from the PAC / SVD)
    // check_gint!("CURMOD_INT", curmod_int);
    // any_remaining |= pre_gint.curmod_int().bit() && post_gint.curmod_int().bit();
    // check_gint!("MODEMIS", modemis);
    // any_remaining |= pre_gint.modemis().bit() && post_gint.modemis().bit();
    // check_gint!("OTGINT", otgint);
    // any_remaining |= pre_gint.otgint().bit() && post_gint.otgint().bit();
    // check_gint!("SOF", sof);
    // any_remaining |= pre_gint.sof().bit() && post_gint.sof().bit();
    // check_gint!("RXFLVI", rxflvi);
    // any_remaining |= pre_gint.rxflvi().bit() && post_gint.rxflvi().bit();
    // check_gint!("NPTXFEMP", nptxfemp);
    // any_remaining |= pre_gint.nptxfemp().bit() && post_gint.nptxfemp().bit();
    // check_gint!("GINNAKEFF", ginnakeff);
    // any_remaining |= pre_gint.ginnakeff().bit() && post_gint.ginnakeff().bit();
    // check_gint!("GOUTNAKEFF", goutnakeff);
    // any_remaining |= pre_gint.goutnakeff().bit() && post_gint.goutnakeff().bit();
    // check_gint!("ERLYSUSP", erlysusp);
    // any_remaining |= pre_gint.erlysusp().bit() && post_gint.erlysusp().bit();
    // check_gint!("USBSUSP", usbsusp);
    // any_remaining |= pre_gint.usbsusp().bit() && post_gint.usbsusp().bit();
    // check_gint!("USBRST", usbrst);
    // any_remaining |= pre_gint.usbrst().bit() && post_gint.usbrst().bit();
    // check_gint!("ENUMDONE", enumdone);
    // any_remaining |= pre_gint.enumdone().bit() && post_gint.enumdone().bit();
    // check_gint!("EOPF", eopf);
    // any_remaining |= pre_gint.eopf().bit() && post_gint.eopf().bit();
    // check_gint!("EPMIS", epmis);
    // any_remaining |= pre_gint.epmis().bit() && post_gint.epmis().bit();
    // check_gint!("IEPINT", iepint);
    // any_remaining |= pre_gint.iepint().bit() && post_gint.iepint().bit();
    // check_gint!("OEPINT", oepint);
    // any_remaining |= pre_gint.oepint().bit() && post_gint.oepint().bit();
    // check_gint!("INCOMPISOIN", incompisoin);
    // any_remaining |= pre_gint.incompisoin().bit() && post_gint.incompisoin().bit();
    // check_gint!("INCOMPIP", incompip);
    // any_remaining |= pre_gint.incompip().bit() && post_gint.incompip().bit();
    // check_gint!("FETSUSP", fetsusp);
    // any_remaining |= pre_gint.fetsusp().bit() && post_gint.fetsusp().bit();
    // check_gint!("RESETDET", resetdet);
    // any_remaining |= pre_gint.resetdet().bit() && post_gint.resetdet().bit();
    // check_gint!("PRTLNT", prtlnt);
    // any_remaining |= pre_gint.prtlnt().bit() && post_gint.prtlnt().bit();
    // check_gint!("HCHLNT", hchlnt);
    // any_remaining |= pre_gint.hchlnt().bit() && post_gint.hchlnt().bit();
    // check_gint!("PTXFEMP", ptxfemp);
    // any_remaining |= pre_gint.ptxfemp().bit() && post_gint.ptxfemp().bit();
    // check_gint!("CONIDSTSCHNG", conidstschng);
    // any_remaining |= pre_gint.conidstschng().bit() && post_gint.conidstschng().bit();
    // check_gint!("DISCONNINT", disconnint);
    // any_remaining |= pre_gint.disconnint().bit() && post_gint.disconnint().bit();
    // check_gint!("SESSREQINT", sessreqint);
    // any_remaining |= pre_gint.sessreqint().bit() && post_gint.sessreqint().bit();
    // check_gint!("WKUPINT", wkupint);
    // any_remaining |= pre_gint.wkupint().bit() && post_gint.wkupint().bit();

    // // also check USB_DEVICE wrapper raw bits numerically (PAC may provide field accessors too)
    // let remaining_dev_int_raw = pre_dev_int_raw & post_dev_int_raw;

    // if any_remaining || remaining_dev_int_raw != 0 {
    //     println!(
    //         "[irq] remaining after handler: USB0 gintmsk={:#010x} USB_DEVICE int_raw={:#010x}",
    //         post_gintmsk, remaining_dev_int_raw
    //     );
    //     // (optionally) clear wrapper or gintsts for debug as before...
    // }

    // dump_usb_mode_and_clear();
}

#[unsafe(no_mangle)]
pub extern "C" fn tusb_esp32_int_enable(
    irq_num: u32,
    handler: extern "C" fn(*mut c_void),
    arg: *mut c_void,
) {
    // println!("[tusb_esp32_int_enable] irq_num={}, handler={:?}, arg={:?}", irq_num, handler, arg);
    assert!(irq_num == esp32s3::Interrupt::USB as u32);

    TUSB_HANDLER.store(handler as usize, Ordering::SeqCst);
    TUSB_HANDLER_ARG.store(arg as usize, Ordering::SeqCst);

    if !TUSB_BOUND.swap(true, Ordering::SeqCst) {
        unsafe {
            esp_hal::interrupt::bind_interrupt(esp32s3::Interrupt::USB, interrupt_trampoline);
        }
    }

    if let Err(err) = esp_hal::interrupt::enable(Interrupt::USB, Priority::Priority2) {
        panic!("Failed to enable USB interrupt: {:?}", err);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tusb_esp32_int_disable(irq_num: u32) {
    esp_hal::interrupt::disable(Cpu::ProCpu, Interrupt::USB);

    // println!("[tusb_esp32_int_disable] irq_num={}", irq_num);
    // assert!(irq_num == esp32s3::Interrupt::USB as u32);

    // TUSB_HANDLER.store(0, Ordering::SeqCst);
    // TUSB_HANDLER_ARG.store(0, Ordering::SeqCst);
}

#[unsafe(no_mangle)]
pub extern "C" fn tusb_esp32_delay_ms(ms: u32) {
    let now = esp_hal::time::Instant::now();
    while now.elapsed().as_millis() < ms as u64 {
        core::hint::spin_loop();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tusb_esp32_dcache_clean(_addr: *const c_void, _size: u32) -> bool {
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn tusb_esp32_dcache_invalidate(_addr: *const c_void, _size: u32) -> bool {
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn tusb_esp32_dcache_clean_invalidate(_addr: *const c_void, _size: u32) -> bool {
    true
}

#[unsafe(no_mangle)]
pub extern "C" fn tusb_time_millis_api() -> u32 {
    static mut PROGRAM_START: Option<esp_hal::time::Instant> = None;
    let program_start = unsafe {
        if matches!(PROGRAM_START, None) {
            PROGRAM_START = Some(esp_hal::time::Instant::now());
        }
        PROGRAM_START.unwrap()
    };
    program_start.elapsed().as_millis() as u32
}

pub type HidReportCallback = fn(dev_addr: u8, instance: u8, report: *const u8, len: u16);
static RUST_HID_HANDLER: AtomicUsize = AtomicUsize::new(0);

pub fn set_rust_hid_report_callback(cb: Option<HidReportCallback>) -> Option<HidReportCallback> {
    let prev = RUST_HID_HANDLER.swap(cb.map(|f| f as usize).unwrap_or(0), Ordering::SeqCst);
    if prev == 0 {
        None
    } else {
        // SAFETY: we only store function pointers of type `HidReportCallback` here.
        Some(unsafe { core::mem::transmute(prev) })
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tuh_hid_report_received_cb(
    dev_addr: u8,
    instance: u8,
    report: *const u8,
    len: u16,
) {
    // Forward to any registered pure-Rust callback first (if present).
    let rust_h = RUST_HID_HANDLER.load(Ordering::SeqCst);
    if rust_h != 0 {
        let cb: HidReportCallback = unsafe { core::mem::transmute(rust_h) };
        cb(dev_addr, instance, report, len);
    }

    // continue to request to receive report
    if !unsafe { tinyusb_sys::tuh_hid_receive_report(dev_addr, instance) } {
        println!("Error: cannot request report");
    }
}

unsafe fn cstr_to_str(ptr: *const c_char) -> &'static str {
    if ptr.is_null() {
        return "";
    }
    let mut len = 0usize;
    // find NUL terminator
    loop {
        if *ptr.add(len) == 0 {
            break;
        }
        len += 1;
    }
    let slice = core::slice::from_raw_parts(ptr as *const u8, len);
    core::str::from_utf8_unchecked(slice)
}

// Helper: read n usize-sized words from args pointer and assemble into u64 (little-endian)
fn read_u64_from_words(words: &[usize]) -> u64 {
    let mut v: u64 = 0;
    let word_bytes = core::mem::size_of::<usize>();
    // little-endian target assumed
    for (i, &w) in words
        .iter()
        .enumerate()
        .take((8 + word_bytes - 1) / word_bytes)
    {
        let shift = (i * word_bytes) * 8;
        v |= (w as u64) << shift;
    }
    v
}

// Parse the C-like format string and for every conversion specifier consume arguments
// from args_ptr (pointer to an array of usize words) and print interpreted values.
// This is intentionally conservative and aims to support common specifiers for debugging:
// %d %i %u %x %X %p %s %c %f (double) and length modifiers l, ll.
fn print_parsed_args(tag: &str, fmt: &str, args: &mut core::ffi::VaListImpl<'_>) {
    let mut param_idx = 0usize;

    let mut chars = fmt.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '%' {
            continue;
        }
        // handle "%%"
        if let Some('%') = chars.peek() {
            let _ = chars.next();
            continue;
        }

        // skip flags
        while matches!(chars.peek(), Some(&c) if "-+ #0".contains(c)) {
            let _ = chars.next();
        }
        // skip width (digits or *)
        if let Some(&c) = chars.peek() {
            if c == '*' {
                // consume the width-argument
                let _: i32 = unsafe { args.arg() };
                let _ = chars.next();
            } else {
                while matches!(chars.peek(), Some(&d) if d.is_ascii_digit()) {
                    let _ = chars.next();
                }
            }
        }
        // skip precision
        if let Some('.') = chars.peek() {
            let _ = chars.next();
            if let Some(&c) = chars.peek() {
                if c == '*' {
                    // consumes an int argument
                    let _: i32 = unsafe { args.arg() };
                    let _ = chars.next();
                } else {
                    while matches!(chars.peek(), Some(&d) if d.is_ascii_digit()) {
                        let _ = chars.next();
                    }
                }
            }
        }

        // length modifiers: support hh, h, l, ll, L, z, t
        let mut length_ll = false;
        let mut length_l = false;
        if let Some(&c) = chars.peek() {
            if c == 'h' {
                let _ = chars.next();
                if let Some(&'h') = chars.peek() {
                    let _ = chars.next();
                }
            } else if c == 'l' {
                let _ = chars.next();
                if let Some(&'l') = chars.peek() {
                    let _ = chars.next();
                    length_ll = true;
                } else {
                    length_l = true;
                }
            } else if c == 'L' || c == 'z' || c == 't' {
                let _ = chars.next();
            }
        }

        // conversion specifier
        let spec = chars.next();
        if spec.is_none() {
            break;
        }
        let spec = spec.unwrap();

        match spec {
            'd' | 'i' => {
                // signed integer: width depends on length modifiers
                if length_ll {
                    // 64-bit
                    let i: i64 = unsafe { args.arg() };
                    println!("  [{}] {}", param_idx, i);
                } else if length_l {
                    // long on 64-bit
                    let i: isize = unsafe { args.arg() };
                    println!("  [{}] {}", param_idx, i);
                } else {
                    // int promoted to int/unsigned fits in usize word
                    let i: i32 = unsafe { args.arg() };
                    println!("  [{}] {}", param_idx, i);
                }
            }
            'u' | 'x' | 'X' => {
                // unsigned integer / hex
                if length_ll {
                    // 64-bit
                    let u: u64 = unsafe { args.arg() };
                    if spec == 'u' {
                        println!("  [{}] {}", param_idx, u);
                    } else {
                        println!("  [{}] {:#x}", param_idx, u);
                    }
                } else if length_l {
                    // long on 64-bit
                    let u: usize = unsafe { args.arg() };
                    if spec == 'u' {
                        println!("  [{}] {}", param_idx, u);
                    } else {
                        println!("  [{}] {:#x}", param_idx, u);
                    }
                } else {
                    // int promoted to int/unsigned fits in usize word
                    let u: u32 = unsafe { args.arg() };
                    if spec == 'u' {
                        println!("  [{}] {}", param_idx, u);
                    } else {
                        println!("  [{}] {:#x}", param_idx, u);
                    }
                }
            }
            'p' => {
                // pointer
                let p: *const c_void = unsafe { args.arg() };
                println!("  [{}] {:?}", param_idx, p);
            }
            's' => {
                // string pointer
                let s: *const c_char = unsafe { args.arg() };
                let s = unsafe { cstr_to_str(s) };
                println!("  [{}] \"{}\"", param_idx, s);
            }
            'c' => {
                // char promoted to int
                let c: i32 = unsafe { args.arg() };
                println!("  [{}] '{}'", param_idx, (c as u8) as char);
            }
            'f' | 'F' | 'g' | 'G' | 'e' | 'E' => {
                // double (promoted)
                let f: f64 = unsafe { args.arg() };
                println!("  [{}] {}", param_idx, f);
            }
            // fallback: treat as pointer-sized raw value
            _ => {}
        }

        param_idx += 1;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tusb_esp32_logv(tag: *const c_char, fmt: *const c_char, mut args: ...) {
    let tag = unsafe { cstr_to_str(tag) };
    let fmt = unsafe { cstr_to_str(fmt) };
    print!("[{}] {}", tag, fmt);
    if !fmt.ends_with("\n") {
        println!("");
    }
    print_parsed_args(tag, fmt, &mut args);
    println!("");
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn tusb_esp32_early_logv(
    tag: *const c_char,
    fmt: *const c_char,
    mut args: ...
) {
    let tag = unsafe { cstr_to_str(tag) };
    let fmt = unsafe { cstr_to_str(fmt) };
    print!("[{}] {}", tag, fmt);
    if !fmt.ends_with("\n") {
        println!("");
    }
    print_parsed_args(tag, fmt, &mut args);
    println!("");
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printf(fmt: *const c_char, mut args: ...) -> i32 {
    let fmt = unsafe { cstr_to_str(fmt) };
    print!("{}", fmt);
    if !fmt.ends_with("\n") {
        println!("");
    }
    print_parsed_args("", fmt, &mut args);
    println!("");
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn puts(s: *const c_char) -> i32 {
    let s = unsafe { cstr_to_str(s) };
    print!("{}", s);
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn putchar(c: i32) -> i32 {
    print!("{}", (c as u8) as char);
    c
}

#[unsafe(no_mangle)]
pub static _ctype_: [u8; 1] = [0];
