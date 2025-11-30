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

    // Magic RAM location and value for App2 (matches bootloader noinit section)
    const MAGIC_ADDR: *mut u32 = 0x2001_FFF8 as *mut u32;
    const MAGIC_APP2: u32 = 0xCAFE_BABE;

    // Write magic value to RAM
    write_volatile(MAGIC_ADDR, MAGIC_APP2);

    // Memory barrier
    cortex_m::asm::dsb();

    // Trigger system reset using cortex-m API
    cortex_m::peripheral::SCB::sys_reset();
}
#[rtic::app(device = stm32f4xx_hal::pac, peripherals = true)]
mod app {

    use core::fmt::Write;
    use stm32f4xx_hal::{
        gpio::{self, Edge, Input, Output, PushPull},
        pac::{TIM1, USART2},
        prelude::*,
        rcc::Config,
        serial::{config::Config as SerialConfig, Serial},
        timer,
    };

    use crate::jump_to_other;

    const APP2_ADDR: u32 = 0x08024000; // App2 new address after bootloader

    // Resources shared between tasks
    #[shared]
    struct Shared {
        delayval: u32,
    }

    // Local resources to specific tasks (cannot be shared)
    #[local]
    struct Local {
        button: gpio::PA0<Input>,
        led: gpio::PC13<Output<PushPull>>,
        delay: timer::DelayMs<TIM1>,
        uart: Serial<USART2>,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local) {
        let mut dp = ctx.device;

        // Configure and obtain handle for delay abstraction
        // 1) Promote RCC structure to HAL to be able to configure clocks
        let rcc = dp.RCC.constrain();
        let mut rcc = rcc.freeze(Config::hse(25.MHz()));

        // 3) Create delay handle
        let delay = dp.TIM1.delay_ms(&mut rcc);

        // Configure the LED pin as a push pull ouput and obtain handle
        // On the Blackpill STM32F411CEU6 there is an on-board LED connected to pin PC13
        // 1) Promote the GPIOC PAC struct
        let gpioc = dp.GPIOC.split(&mut rcc);

        // 2) Configure PORTC OUTPUT Pins and Obtain Handle
        let led = gpioc.pc13.into_push_pull_output();

        // Configure the button pin as input and obtain handle
        // On the Blackpill STM32F411CEU6 there is a button connected to pin PA0
        // 1) Promote the GPIOA PAC struct
        let gpioa: gpio::gpioa::Parts = dp.GPIOA.split(&mut rcc);
        // 2) Configure Pin and Obtain Handle
        let mut button = gpioa.pa0.into_pull_up_input();

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
        writeln!(uart, "\r\n=== APP1 STARTING ===").ok();

        // Configure Button Pin for Interrupts
        // 1) Promote SYSCFG structure to HAL to be able to configure interrupts
        let mut syscfg = dp.SYSCFG.constrain(&mut rcc);
        // 2) Make button an interrupt source
        button.make_interrupt_source(&mut syscfg);
        // 3) Configure the interruption to be triggered on a rising edge
        button.trigger_on_edge(&mut dp.EXTI, Edge::Rising);
        // 4) Enable gpio interrupt for button
        button.enable_interrupt(&mut dp.EXTI);

        writeln!(uart, "APP1: Init complete - button interrupt enabled").ok();
        writeln!(uart, "APP1: Press button to switch to APP2").ok();

        (
            // Initialization of shared resources
            Shared { delayval: 2000_u32 },
            // Initialization of task local resources
            Local {
                button,
                led,
                delay,
                uart,
            },
        )
    }

    // Background task, runs whenever no other tasks are running
    #[idle(local = [led, delay], shared = [delayval])]
    fn idle(mut ctx: idle::Context) -> ! {
        let led = ctx.local.led;
        let delay = ctx.local.delay;
        loop {
            // First fast blink
            led.set_high();
            delay.delay_ms(150u32);
            led.set_low();
            delay.delay_ms(50u32);

            // Second fast blink
            led.set_high();
            delay.delay_ms(150u32);
            led.set_low();
            delay.delay_ms(50u32);

            // Long pause with LED ON
            led.set_high();
            delay.delay_ms(ctx.shared.delayval.lock(|del| *del));
            led.set_low();
        }
    }

    #[task(binds = EXTI0, local = [button, uart], shared=[delayval])]
    fn gpio_interrupt_handler(ctx: gpio_interrupt_handler::Context) {
        writeln!(ctx.local.uart, "APP1: Button pressed! Switching to APP2...").ok();
        ctx.local.button.clear_interrupt_pending_bit();

        // Jump to the other application
        unsafe {
            jump_to_other(APP2_ADDR);
        }
    }
}
