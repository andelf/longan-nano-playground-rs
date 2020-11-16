#![no_std]
#![no_main]
#![feature(asm)]

use panic_halt as _;

use core::fmt::Write;

// for LCD
use embedded_graphics::fonts::{Font8x16, Text};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::{primitive_style, text_style};
// gd32vf103_pac
use gd32vf103xx_hal::delay;
use gd32vf103xx_hal::pac;
use gd32vf103xx_hal::prelude::*;
use longan_nano_playground::ByteMutWriter;
use longan_nano_playground::{lcd, lcd_pins};
use riscv_rt::entry;

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

    // 160 / 8 = 20 chars, per line
    let mut buf = [0u8; 20 * 5];
    let mut buf = ByteMutWriter::new(&mut buf[..]);

    // delay using mcycle
    let mut delay = delay::McycleDelay::new(&rcu.clocks);

    // Create a text at position (20, 30) and draw it using style defined above
    Text::new(" Hello from Rust! ", Point::new(0, 0))
        .into_styled(style)
        .draw(&mut lcd)
        .unwrap();

    delay.delay_ms(2_000_u16);

    buf.clear();
    let _ = writeln!(buf, "misa:      {:08x}", misa());
    let _ = writeln!(buf, "mvendorid: {:08x}", mvendorid());
    let _ = writeln!(buf, "marchid:   {:08x}", marchid());
    let _ = writeln!(buf, "mimpid:    {:08x}", mimpid());
    let _ = writeln!(buf, "mhartid:   {:08x}", mhartid());

    Text::new(buf.as_str(), Point::new(0, 0))
        .into_styled(style)
        .draw(&mut lcd)
        .unwrap();

    loop {}
}

fn misa() -> u32 {
    let mut x: u32 = 0;
    unsafe {
        asm!(
            "csrr a0, {mcsr}",
            "mv {x}, a0",
            x = inout(reg) x,
            mcsr = const 0x301,
            options(nostack)
        );
    }
    x
}

fn mvendorid() -> u32 {
    let mut x: u32 = 0;
    unsafe {
        asm!(
            "csrr a0, {mcsr}",
            "mv {x}, a0",
            x = inout(reg) x,
            mcsr = const 0xf11,
            options(nostack)
        );
    }
    x
}

fn marchid() -> u32 {
    let mut x: u32 = 0;
    unsafe {
        asm!(
            "csrr a0, {mcsr}",
            "mv {x}, a0",
            x = inout(reg) x,
            mcsr = const 0xf12,
            options(nostack)
        );
    }
    x
}

fn mimpid() -> u32 {
    let mut x: u32 = 0;
    unsafe {
        asm!(
            "csrr a0, {mcsr}",
            "mv {x}, a0",
            x = inout(reg) x,
            mcsr = const 0xf13,
            options(nostack)
        );
    }
    x
}
fn mhartid() -> u32 {
    let mut x: u32 = 0;
    unsafe {
        asm!(
            "csrr a0, {mcsr}",
            "mv {x}, a0",
            x = inout(reg) x,
            mcsr = const 0xf14,
            options(nostack)
        );
    }
    x
}
