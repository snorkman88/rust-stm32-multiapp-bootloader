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
    
    // Trigger system reset - bootloader will see magic and boot app2
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

    use stm32f4xx_hal::{
        gpio::{self, Edge, Input, Output, PushPull},
        pac::TIM1,
        prelude::*,
        rcc::Config,
        timer,
    };

    use defmt_rtt as _;

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

        // Configure Button Pin for Interrupts
        // 1) Promote SYSCFG structure to HAL to be able to configure interrupts
        let mut syscfg = dp.SYSCFG.constrain(&mut rcc);
        // 2) Make button an interrupt source
        button.make_interrupt_source(&mut syscfg);
        // 3) Configure the interruption to be triggered on a rising edge
        button.trigger_on_edge(&mut dp.EXTI, Edge::Rising);
        // 4) Enable gpio interrupt for button
        button.enable_interrupt(&mut dp.EXTI);
        // 5) CRITICAL: Explicitly unmask EXTI0 in NVIC after jump
        unsafe {
            use cortex_m::peripheral::NVIC;
            use stm32f4xx_hal::pac::Interrupt;
            use core::ptr::read_volatile;
            
            // Check SYSCFG EXTICR1 (controls EXTI0-3) - should be 0x0 for PA0
            const SYSCFG_EXTICR1: *const u32 = 0x4001_3808 as *const u32;
            let exticr1 = read_volatile(SYSCFG_EXTICR1);
            
            // Check EXTI registers
            const EXTI_IMR: *const u32 = 0x4001_3C00 as *const u32;
            const EXTI_RTSR: *const u32 = 0x4001_3C08 as *const u32;
            let exti_imr = read_volatile(EXTI_IMR);
            let exti_rtsr = read_volatile(EXTI_RTSR);
            
            defmt::warn!("SYSCFG_EXTICR1={:#010x} (should be 0x0 for GPIOA)", exticr1);
            defmt::warn!("EXTI_IMR={:#010x} EXTI_RTSR={:#010x}", exti_imr, exti_rtsr);
            
            NVIC::unmask(Interrupt::EXTI0);
            
            const NVIC_ISER0: *const u32 = 0xE000_E100 as *const u32;
            let nvic_iser = read_volatile(NVIC_ISER0);
            defmt::warn!("NVIC_ISER0={:#010x} (bit0 should be 1)", nvic_iser);
        }

        defmt::warn!("=== APP1 INITIALIZATION COMPLETE ===");

        (
            // Initialization of shared resources
            Shared { delayval: 2000_u32 },
            // Initialization of task local resources
            Local { button, led, delay },
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
            delay.delay_ms(50u32);
            led.set_low();
            delay.delay_ms(50u32);

            // Second fast blink
            led.set_high();
            delay.delay_ms(50u32);
            led.set_low();
            delay.delay_ms(50u32);

            // Long pause with LED ON
            led.set_high();
            delay.delay_ms(ctx.shared.delayval.lock(|del| *del));
            led.set_low();
        }
    }

    #[task(binds = EXTI0, local = [button], shared=[delayval])]
    fn gpio_interrupt_handler(ctx: gpio_interrupt_handler::Context) {
        defmt::warn!("!!! BUTTON INTERRUPT FIRED !!!");
        ctx.local.button.clear_interrupt_pending_bit();

        // Jump to the other application
        unsafe {
            jump_to_other(APP2_ADDR);
        }
    }
}
