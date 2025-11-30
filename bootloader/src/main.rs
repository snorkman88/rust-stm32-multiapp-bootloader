#![no_std]
#![no_main]

use core::ptr::{read_volatile, write_volatile};
use cortex_m_rt::entry;
use panic_halt as _;

// Magic value stored in noinit section (survives reset)
#[link_section = ".noinit"]
static mut MAGIC_VALUE: u32 = 0;

// Magic values for app selection
const MAGIC_APP1: u32 = 0xDEAD_BEEF;
const MAGIC_APP2: u32 = 0xCAFE_BABE;

// Application base addresses (after 16KB bootloader)
const APP1_ADDR: u32 = 0x0800_4000; // 16KB offset
const APP2_ADDR: u32 = 0x0802_4000; // 16KB + 128KB offset

/// Jumps to an application at the given address
///
/// # Safety
/// This must point to a valid application with proper vector table
unsafe fn jump_to_app(addr: u32) -> ! {
    use core::ptr::read_volatile;

    // Read the initial stack pointer and reset vector from the app's vector table
    let msp = read_volatile(addr as *const u32);
    let reset_vector = read_volatile((addr + 4) as *const u32);

    // Set VTOR to point to the application's vector table
    const SCB_VTOR: *mut u32 = 0xE000_ED08 as *mut u32;
    core::ptr::write_volatile(SCB_VTOR, addr);

    // Memory barriers
    cortex_m::asm::dsb();
    cortex_m::asm::isb();

    // Jump to the application using bootload
    cortex_m::asm::bootload(addr as *const u32)
}

#[entry]
fn main() -> ! {
    // Read the magic value from noinit RAM using raw pointer
    let magic_ptr = unsafe { core::ptr::addr_of!(MAGIC_VALUE) as *const u32 };
    let magic = unsafe { read_volatile(magic_ptr) };

    // Clear the magic value so default boot works after power cycle
    let magic_ptr_mut = unsafe { core::ptr::addr_of_mut!(MAGIC_VALUE) };
    unsafe {
        write_volatile(magic_ptr_mut, 0);
    }

    // Decide which app to boot based on magic value
    let app_addr = match magic {
        MAGIC_APP2 => APP2_ADDR,
        MAGIC_APP1 => APP1_ADDR,
        _ => APP1_ADDR, // Default to App1
    };

    // Jump to the selected application
    unsafe {
        jump_to_app(app_addr);
    }
}
