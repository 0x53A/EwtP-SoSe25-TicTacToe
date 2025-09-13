use core::ffi::c_char;
use core::ffi::c_void;
use core::sync::atomic::AtomicBool;
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};

use esp_hal::interrupt::Priority;
use esp_hal::system::Cpu;
use esp_println::{print, println};
use esp32s3::Interrupt;

// store the registered handler and arg so the trampoline can forward the interrupt
static TUSB_HANDLER: AtomicUsize = AtomicUsize::new(0);
static TUSB_HANDLER_ARG: AtomicUsize = AtomicUsize::new(0);

static TUSB_BOUND: AtomicBool = AtomicBool::new(false);

pub static INTERRUPT_COUNTER: AtomicU32 = AtomicU32::new(0);

pub unsafe extern "C" fn interrupt_trampoline() {
    unsafe {
        INTERRUPT_COUNTER.fetch_add(1, Ordering::SeqCst);

        // forward to saved handler
        let h = TUSB_HANDLER.load(Ordering::SeqCst);
        if h == 0 {
            return;
        }
        let arg = TUSB_HANDLER_ARG.load(Ordering::SeqCst) as *mut c_void;
        let handler: extern "C" fn(*mut c_void) = core::mem::transmute(h);
        handler(arg);
    }
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
pub extern "C" fn tusb_esp32_int_disable(_irq_num: u32) {
    esp_hal::interrupt::disable(Cpu::ProCpu, Interrupt::USB);
}

#[unsafe(no_mangle)]
pub extern "C" fn tusb_esp32_delay_ms(ms: u32) {
    let now = esp_hal::time::Instant::now();
    while now.elapsed().as_millis() < ms as u64 {
        core::hint::spin_loop();
    }
}

// #[unsafe(no_mangle)]
// pub extern "C" fn tusb_esp32_dcache_clean(_addr: *const c_void, _size: u32) -> bool {
//     true
// }

// #[unsafe(no_mangle)]
// pub extern "C" fn tusb_esp32_dcache_invalidate(_addr: *const c_void, _size: u32) -> bool {
//     true
// }

// #[unsafe(no_mangle)]
// pub extern "C" fn tusb_esp32_dcache_clean_invalidate(_addr: *const c_void, _size: u32) -> bool {
//     true
// }

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
            tinyusb_sys::tuh_hid_receive_report(daddr, 0);
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
        println!(
            "cfg header fetch result for device {} = {:?}",
            daddr, hdr_res
        );
        if hdr_res != tinyusb_sys::xfer_result_t::XFER_RESULT_SUCCESS {
            println!(
                "Failed to fetch config header for device {} (res={:?})",
                daddr, hdr_res
            );
            return;
        }

        // Parse wTotalLength from the 9-byte header (little-endian at offset 2)
        let total_len = (header[2] as usize) | ((header[3] as usize) << 8);
        if total_len == 0 {
            println!(
                "Device {}: wTotalLength == 0 in header — can't read full config",
                daddr
            );
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
        println!(
            "cfg full fetch result for device {} = {:?} (requested {})",
            daddr, cfg_res, to_read
        );
        if cfg_res == tinyusb_sys::xfer_result_t::XFER_RESULT_SUCCESS {
            // parse and dump
            parse_and_print_hid_poll_interval(daddr, &cfg_buf[..to_read]);
            dump_and_find_interrupt_endpoints(daddr, &cfg_buf[..to_read]);
        } else {
            println!(
                "Failed to get configuration descriptor for addr {} (res={:?})",
                daddr, cfg_res
            );
        }
    }
}

/// Callback invoked by tinyusb when a device is unmounted.
#[unsafe(no_mangle)]
extern "C" fn tuh_umount_cb(daddr: u8) {
    println!("Device removed, address = {}", daddr);
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

    // let proto = unsafe { tinyusb_sys::tuh_hid_interface_protocol(dev_addr, instance) };
    // if proto == tinyusb_sys::hid_interface_protocol_enum_t::HID_ITF_PROTOCOL_KEYBOARD as u8 {
    //     println!("proto == keyboard");

    //     if len as usize >= core::mem::size_of::<tinyusb_sys::hid_keyboard_report_t>() {
    //         let report = unsafe { &*(report as *const tinyusb_sys::hid_keyboard_report_t) };
    //         process_kbd_report(dev_addr, report);
    //     } else {
    //         println!("Warning: HID keyboard report too short: {}", len);
    //     }
    // } else {
    //     println!("HID protocol not handled: {:?}", proto);
    // }
}

unsafe fn cstr_to_str(ptr: *const c_char) -> &'static str {
    unsafe {
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
}

// Parse the C-like format string and for every conversion specifier consume arguments
// from args_ptr (pointer to an array of usize words) and print interpreted values.
// This is intentionally conservative and aims to support common specifiers for debugging:
// %d %i %u %x %X %p %s %c %f (double) and length modifiers l, ll.
fn print_parsed_args(_tag: &str, fmt: &str, args: &mut core::ffi::VaListImpl<'_>) {
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

#[allow(non_upper_case_globals)]
#[unsafe(no_mangle)]
pub static _ctype_: [u8; 1] = [0];

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
    unsafe {
        let mut result: tinyusb_sys::xfer_result_t =
            tinyusb_sys::xfer_result_t::XFER_RESULT_INVALID;
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
