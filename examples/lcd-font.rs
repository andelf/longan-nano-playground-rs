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

/*
ProFont7Point	The 7 point size with a character size of 5x9 pixels.
ProFont9Point	The 9 point size with a character size of 6x11 pixels.
ProFont10Point	The 10 point size with a character size of 7x13 pixels.
ProFont12Point	The 12 point size with a character size of 8x15 pixels.
ProFont14Point	The 14 point size with a character size of 10x18 pixels.
ProFont18Point	The 18 point size with a character size of 12x22 pixels.
ProFont24Point	The 24 point size with a character size of 16x30 pixels.
*/
// use profont::ProFont10Point;

use embedded_graphics::fonts::Font;
use embedded_graphics::geometry::Size;

#[derive(Clone, Copy)]
pub struct ChnFont;

impl Font for ChnFont {
    const FONT_IMAGE: &'static [u8] = include_bytes!("../font.raw");
    const FONT_IMAGE_WIDTH: u32 = 16;
    const CHARACTER_SIZE: Size = Size::new(16, 16);
    const VARIABLE_WIDTH: bool = true;

    fn char_offset(c: char) -> u32 {
        unreachable!()
    }

    fn char_width(c: char) -> u32 {
        if (c as u16) < 128 {
            8
        } else {
            16
        }
    }

    fn character_pixel(c: char, x: u32, y: u32) -> bool {
        if (c as u16) < 128 {
            let map = &Self::FONT_IMAGE[16 * (c as usize)..16 + (c as usize) * 16];
            return (map[y as usize] & (1 << (7 - x))) != 0;
        }

        const start: usize = 128 * 16;
        const step: usize = 2 * 16;
        let map = match c {
            '卧' => &Self::FONT_IMAGE[start..start + 1 * step],
            '槽' => &Self::FONT_IMAGE[start + 1 * step..start + 2 * step],
            '艹' => &Self::FONT_IMAGE[start + 2 * step..start + 3 * step],
            _ => {
                return false;
            }
        };

        if x >= 8 {
            (map[y as usize * 2 + 1] & (1 << (15 - x))) != 0
        } else {
            (map[y as usize * 2] & (1 << (7 - x))) != 0
        }
    }
}

// rtc
use gd32vf103xx_hal::rtc::Rtc;

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

    // Clear screen
    Rectangle::new(Point::new(0, 0), Point::new(width - 1, height - 1))
        .into_styled(primitive_style!(fill_color = Rgb565::BLACK))
        .draw(&mut lcd)
        .unwrap();

    let style = text_style!(
        font = ChnFont, // Font6x8,
        text_color = Rgb565::WHITE,
        background_color = Rgb565::BLACK
    );

    // let max_duty = pwm.try_get_max_duty().unwrap();

    // 160 / 8 = 20 chars, per line
    let mut buf = [0u8; 20];
    let mut buf = ByteMutWriter::new(&mut buf[..]);

    // Create a text at position (20, 30) and draw it using style defined above
    Text::new("卧槽艹~ABCD卧卧槽", Point::new(0, 0))
        .into_styled(style)
        .draw(&mut lcd)
        .unwrap();

    loop {}
}
