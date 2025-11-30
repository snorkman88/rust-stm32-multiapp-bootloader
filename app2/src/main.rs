//#![deny(unsafe_code)]
#![deny(warnings)]
#![no_main]
#![no_std]

use panic_halt as _;

/// Jumps to another application via bootloader
///
/// # Safety
/// Triggers a system reset after writing magic value to RAM
pub unsafe fn jump_to_other(_addr: u32) -> ! {
    use core::ptr::write_volatile;

    // Magic RAM location and value for App1 (matches bootloader noinit section)
    const MAGIC_ADDR: *mut u32 = 0x2001_FFF8 as *mut u32;
    const MAGIC_APP1: u32 = 0xDEAD_BEEF;

    // Write magic value to RAM
    write_volatile(MAGIC_ADDR, MAGIC_APP1);

    // Memory barrier
    cortex_m::asm::dsb();

    // Trigger system reset - bootloader will see magic and boot app1
    const SCB_AIRCR: *mut u32 = 0xE000_ED0C as *mut u32;
    const AIRCR_VECTKEY: u32 = 0x05FA << 16;
    const AIRCR_SYSRESETREQ: u32 = 1 << 2;

    write_volatile(SCB_AIRCR, AIRCR_VECTKEY | AIRCR_SYSRESETREQ);
    cortex_m::asm::dsb();

    // Wait for reset
    loop {
        cortex_m::asm::nop();
    }
}

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true)]
mod app {
    use defmt_rtt as _;
    use stm32f4xx_hal::{
        gpio::{self, Input, Output, PushPull},
        pac::TIM1,
        prelude::*,
        rcc::Config,
        timer,
    };

    use crate::jump_to_other;

    const APP1_ADDR: u32 = 0x08004000; // App1 new address after bootloader

    #[shared]
    struct Shared {
        delayval: u32,
    }

    #[local]
    struct Local {
        button: gpio::PA0<Input>,
        led: gpio::PC13<Output<PushPull>>,
        delay: timer::DelayMs<TIM1>,
        last_button_state: bool,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local) {
        let dp = ctx.device;
        let rcc = dp.RCC.constrain();
        let mut rcc = rcc.freeze(Config::hse(25.MHz()));
        let delay = dp.TIM1.delay_ms(&mut rcc);
        let gpioc = dp.GPIOC.split(&mut rcc);
        let led = gpioc.pc13.into_push_pull_output();
        let gpioa: gpio::gpioa::Parts = dp.GPIOA.split(&mut rcc);
        let button = gpioa.pa0.into_pull_up_input();
        let last_button_state = button.is_high();
        defmt::warn!("=== APP2 INIT COMPLETE ===");
        (
            Shared { delayval: 50_u32 },
            Local {
                button,
                led,
                delay,
                last_button_state,
            },
        )
    }

    #[idle(local = [led, delay, button, last_button_state], shared = [delayval])]
    fn idle(mut ctx: idle::Context) -> ! {
        let led = ctx.local.led;
        let delay = ctx.local.delay;
        let button = ctx.local.button;
        let last_button_state = ctx.local.last_button_state;

        loop {
            let current_button_state = button.is_high();

            // Detect rising edge (button press) - JUMP TO APP1
            if current_button_state && !*last_button_state {
                // Jump to app1
                unsafe {
                    jump_to_other(APP1_ADDR);
                }
            }

            *last_button_state = current_button_state;

            // Blink LED
            led.set_high();
            delay.delay_ms(ctx.shared.delayval.lock(|del| *del));
            led.set_low();
            delay.delay_ms(ctx.shared.delayval.lock(|del| *del));
        }
    }
}
