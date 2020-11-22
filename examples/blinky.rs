//! Blinky for Wio Lite RISC-V

#![no_std]
#![no_main]
#![feature(asm)]

use panic_halt as _;

use gd32vf103xx_hal::delay;
use gd32vf103xx_hal::pac;
use gd32vf103xx_hal::prelude::*;
use riscv_rt::entry;

use embedded_hal::digital::v2::ToggleableOutputPin;

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Configure clocks
    let mut rcu = dp
        .RCU
        .configure()
        .ext_hf_clock(8.mhz())
        .sysclk(108.mhz())
        .freeze();

    // let mut afio = dp.AFIO.constrain(&mut rcu);

    let gpioa = dp.GPIOA.split(&mut rcu);
    // let gpiob = dp.GPIOB.split(&mut rcu);
    // let gpioc = dp.GPIOC.split(&mut rcu);

    let mut delay = delay::McycleDelay::new(&rcu.clocks);

    // Wio Lite RISC-V: PA8
    // Longan Nano: PA2  (with RGB PC13, PA1, PA2)
    let mut blue_led = gpioa.pa8.into_push_pull_output();

    loop {
        blue_led.toggle().unwrap();
        delay.delay_ms(500);
    }
}
