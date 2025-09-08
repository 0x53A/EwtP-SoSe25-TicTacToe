use core::ffi::c_char;
use core::ffi::c_void;
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use esp_hal::interrupt::Priority;
use esp_hal::system::Cpu;
use esp_println::{print, println};
use esp32s3::Interrupt;

// store the registered handler and arg so the trampoline can forward the interrupt
static TUSB_HANDLER: AtomicUsize = AtomicUsize::new(0);
static TUSB_HANDLER_ARG: AtomicUsize = AtomicUsize::new(0);

pub unsafe extern "C" fn interrupt_trampoline() {
    // load saved handler and arg and forward the interrupt
    let h = TUSB_HANDLER.load(Ordering::SeqCst);
    if h == 0 {
        return;
    }
    let arg = TUSB_HANDLER_ARG.load(Ordering::SeqCst) as *mut c_void;
    let handler: extern "C" fn(*mut c_void) = unsafe { core::mem::transmute(h) };
    // calling the original handler
    handler(arg);
}

#[unsafe(no_mangle)]
pub extern "C" fn tusb_esp32_int_enable(
    irq_num: u32,
    handler: extern "C" fn(*mut c_void),
    arg: *mut c_void,
) {
    assert!(irq_num == esp32s3::Interrupt::USB as u32);

    TUSB_HANDLER.store(handler as usize, Ordering::SeqCst);
    TUSB_HANDLER_ARG.store(arg as usize, Ordering::SeqCst);

    unsafe {
        esp_hal::interrupt::bind_interrupt(esp32s3::Interrupt::USB, interrupt_trampoline);
    }
    if let Err(err) = esp_hal::interrupt::enable(Interrupt::USB, Priority::Priority2) {
        panic!("Failed to enable USB interrupt: {:?}", err);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn tusb_esp32_int_disable(irq_num: u32) {
    assert!(irq_num == esp32s3::Interrupt::USB as u32);

    TUSB_HANDLER.store(0, Ordering::SeqCst);
    TUSB_HANDLER_ARG.store(0, Ordering::SeqCst);

    esp_hal::interrupt::disable(Cpu::ProCpu, Interrupt::USB);
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

#[unsafe(no_mangle)]
pub extern "C" fn tuh_hid_report_received_cb(
    dev_addr: u8,
    instance: u8,
    report: *const u8,
    len: u16,
) {
    println!(
        "HID report received: dev_addr={}, instance={}, report={:?}, len={}",
        dev_addr, instance, report, len
    );
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
    // print_parsed_args(tag, fmt, &mut args);
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
    // print_parsed_args(tag, fmt, &mut args);
    println!("");
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printf(fmt: *const c_char, mut args: ...) -> i32 {
    let fmt = unsafe { cstr_to_str(fmt) };
    print!("{}", fmt);
    if !fmt.ends_with("\n") {
        println!("");
    }
    // print_parsed_args("", fmt, &mut args);
    println!("");
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn puts(s: *const c_char) -> i32 {
    let s = unsafe { cstr_to_str(s) };
    println!("{}", s);
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn putchar(c: i32) -> i32 {
    println!("{}", (c as u8) as char);
    c
}

#[unsafe(no_mangle)]
pub static _ctype_: [u8; 1] = [0];
