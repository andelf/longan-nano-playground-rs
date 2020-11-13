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
use longan_nano::{lcd, lcd_pins};
use riscv_rt::entry;
#[macro_use(block)]
extern crate nb;

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
        font = Font8x16, // Font6x8,
        text_color = Rgb565::WHITE,
        background_color = Rgb565::BLACK
    );

    // let max_duty = pwm.try_get_max_duty().unwrap();

    // 160 / 8 = 20 chars, per line
    let mut buf = [0u8; 20];
    let mut buf = ByteMutWriter::new(&mut buf[..]);

    // Create a text at position (20, 30) and draw it using style defined above
    /*Text::new(" Hello from Rust! ", Point::new(0, 0))
        .into_styled(style)
        .draw(&mut lcd)
        .unwrap();
    */

    {
        buf.clear();
        write!(
            &mut buf,
            "flash size: {}kb",
            gd32vf103xx_hal::signature::flash_size_kb()
        )
        .unwrap();
    }
    Text::new(buf.as_str(), Point::new(0, 16 * 0))
        .into_styled(style)
        .draw(&mut lcd)
        .unwrap();

    {
        buf.clear();
        write!(
            &mut buf,
            "sram size: {}kb",
            gd32vf103xx_hal::signature::sram_size_kb()
        )
        .unwrap();
    }
    Text::new(buf.as_str(), Point::new(0, 16 * 1))
        .into_styled(style)
        .draw(&mut lcd)
        .unwrap();

    {
        let device_id = gd32vf103xx_hal::signature::device_id();
        for i in 0..3 {
            buf.clear();
            write!(&mut buf, "dev id[{}]: {:x}", i, device_id[i]).unwrap();
            Text::new(buf.as_str(), Point::new(0, 16 * (2 + i as i32)))
                .into_styled(style)
                .draw(&mut lcd)
                .unwrap();
        }
    }

    /*
    Text::new("booting...", Point::new(0, 12 * 5))
        .into_styled(style)
        .draw(&mut lcd)
        .unwrap();
        */

    // rcu.clocks.timer0()
    // 以多高频率触发
    // 0.1s
    let mut timer = timer::Timer::timer0(dp.TIMER0, 20.hz(), &mut rcu);
    // timer.listen(Event::Update);
    for _ in 0..50 {
        let _ = block!(timer.wait());
    }

    // Clear screen
    Rectangle::new(Point::new(0, 0), Point::new(width - 1, height - 1))
        .into_styled(primitive_style!(fill_color = Rgb565::BLACK))
        .draw(&mut lcd)
        .unwrap();

    let mut pmu = dp.PMU;
    let mut bak_dom = dp.BKP.configure(&mut rcu, &mut pmu);
    let rtc = Rtc::rtc(dp.RTC, &mut bak_dom);

    loop {
        let _ = block!(timer.wait());

        buf.clear();
        let ctime = rtc.current_time();
        if ctime > 60 * 60 {
            write!(
                &mut buf,
                " uptime: {}:{:02}:{:02}",
                ctime / 3600,
                ctime % 3600 / 60,
                ctime % 60
            )
            .unwrap();
        } else if ctime > 60 {
            write!(&mut buf, " uptime: {}:{:02}", ctime / 60, ctime % 60).unwrap();
        } else {
            write!(&mut buf, " uptime: {}", ctime).unwrap();
        }
        Text::new(buf.as_str(), Point::new(0, height / 2 - 8))
            .into_styled(style)
            .draw(&mut lcd)
            .unwrap();
    }
}
