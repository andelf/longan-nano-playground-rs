#![no_std]
#![no_main]
#![feature(asm)]

use panic_halt as _;

use core::fmt::Write;
use longan_nano_playground::ByteMutWriter;

use embedded_graphics::fonts::{Font8x16, Text};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::{primitive_style, text_style};
// gd32vf103_pac
use gd32vf103xx_hal::pac;
use gd32vf103xx_hal::prelude::*;
use gd32vf103xx_hal::timer;
use longan_nano_playground::{lcd, lcd_pins};
use riscv_rt::entry;
#[macro_use(block)]
extern crate nb;

use embedded_hal::digital::v2::ToggleableOutputPin;

use gd32vf103xx_hal::delay::McycleDelay;

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
    let mut afio = dp.AFIO.constrain(&mut rcu);

    let gpioa = dp.GPIOA.split(&mut rcu);
    let gpiob = dp.GPIOB.split(&mut rcu);

    let lcd_pins = lcd_pins!(gpioa, gpiob);
    let mut lcd = lcd::configure(dp.SPI0, lcd_pins, &mut afio, &mut rcu);
    let (width, height) = (lcd.size().width as i32, lcd.size().height as i32);

    let mut blue = gpioa.pa2.into_push_pull_output();

    macro_rules! cls {
        () => {
            Rectangle::new(Point::new(0, 0), Point::new(width - 1, height - 1))
                .into_styled(primitive_style!(fill_color = Rgb565::BLACK))
                .draw(&mut lcd)
                .unwrap()
        };
    }
    // Clear screen
    cls!();

    let style = text_style!(
        font = Font8x16, // Font6x8,
        text_color = Rgb565::WHITE,
        background_color = Rgb565::BLACK
    );

    let mut buf = [0u8; 20 * 5];
    let mut buf = ByteMutWriter::new(&mut buf[..]);

    let mut delay = McycleDelay::new(&rcu.clocks);

    // trigger timer at 0.1s interval
    let mut timer = timer::Timer::timer0(dp.TIMER0, 10.hz(), &mut rcu);
    // timer.listen(Event::Update);
    for _ in 0..10 {
        write!(&mut buf, ".").unwrap();
        Text::new(buf.as_str(), Point::new(0, 0))
            .into_styled(style)
            .draw(&mut lcd)
            .unwrap();
        let _ = block!(timer.wait());
    }

    cls!();
    buf.clear();
    let device_id = gd32vf103xx_hal::signature::device_id();
    write!(
        &mut buf,
        "flash size: {}kb\nsram size: {}kb\ndev id[0]: {:x}\ndev id[1]: {:x}\ndev id[2]: {:x}",
        gd32vf103xx_hal::signature::flash_size_kb(),
        gd32vf103xx_hal::signature::sram_size_kb(),
        device_id[0],
        device_id[1],
        device_id[2]
    )
    .unwrap();
    Text::new(buf.as_str(), Point::new(0, 0))
        .into_styled(style)
        .draw(&mut lcd)
        .unwrap();

    delay.delay_ms(1000);

    // Clear screen
    cls!();
    buf.clear();
    write!(&mut buf, "led blinky").unwrap();
    Text::new(buf.as_str(), Point::new(0, 0))
        .into_styled(style)
        .draw(&mut lcd)
        .unwrap();

    loop {
        blue.toggle().unwrap();
        delay.delay_ms(200);
    }
}
