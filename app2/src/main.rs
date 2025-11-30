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

    // Trigger system reset using cortex-m API
    cortex_m::peripheral::SCB::sys_reset();
}

#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true)]
mod app {
    use core::fmt::Write;
    use stm32f4xx_hal::{
        gpio::{self, Input, Output, PushPull},
        pac::{TIM1, USART2},
        prelude::*,
        rcc::Config,
        serial::{config::Config as SerialConfig, Serial},
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
        uart: Serial<USART2>,
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

        // Configure UART2 for logging (PA2=TX, PA3=RX)
        let tx_pin = gpioa.pa2.into_alternate();
        let rx_pin = gpioa.pa3.into_alternate();
        let mut uart = Serial::new(
            dp.USART2,
            (tx_pin, rx_pin),
            SerialConfig::default().baudrate(115200.bps()),
            &mut rcc,
        )
        .unwrap();
        writeln!(uart, "\r\n=== APP2 STARTING ===").ok();
        writeln!(uart, "APP2: Init complete - fast blinker mode").ok();
        writeln!(uart, "APP2: Press button to switch to APP1").ok();
        (
            Shared { delayval: 50_u32 },
            Local {
                button,
                led,
                delay,
                last_button_state,
                uart,
            },
        )
    }

    #[idle(local = [led, delay, button, last_button_state, uart], shared = [delayval])]
    fn idle(mut ctx: idle::Context) -> ! {
        let led = ctx.local.led;
        let delay = ctx.local.delay;
        let button = ctx.local.button;
        let last_button_state = ctx.local.last_button_state;
        let uart = ctx.local.uart;

        loop {
            let current_button_state = button.is_high();

            // Detect rising edge (button press) - JUMP TO APP1
            if current_button_state && !*last_button_state {
                writeln!(uart, "APP2: Button pressed! Switching to APP1...").ok();
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
