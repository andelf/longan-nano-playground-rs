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
use embedded_hal::digital::v2::OutputPin;
// gd32vf103_pac
use gd32vf103xx_hal::pac;
use gd32vf103xx_hal::prelude::*;
// use gd32vf103xx_hal::timer;
use longan_nano::{lcd, lcd_pins};
use riscv_rt::entry;

use embedded_graphics::image::{Image, ImageRaw};

use gd32vf103xx_hal::delay;

// spi
use gd32vf103xx_hal::spi::{Spi, MODE_0};

// sdcard
use embedded_sdmmc as sdmmc;

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
            cls!(Rgb565::BLACK)
        };
        ($color:path) => {
            Rectangle::new(Point::new(0, 0), Point::new(width - 1, height - 1))
                .into_styled(primitive_style!(fill_color = $color))
                .draw(&mut lcd)
                .unwrap()
        };
    }

    cls!();

    let style = text_style!(
        font = Font8x16, // Font6x8,
        text_color = Rgb565::WHITE,
        background_color = Rgb565::BLACK
    );

    // 160 / 8 = 20 chars, per line
    let mut buf = [0u8; 20 * 5];
    let mut buf = ByteMutWriter::new(&mut buf[..]);

    Text::new(" Hello from Rust! ", Point::new(0, 0))
        .into_styled(style)
        .draw(&mut lcd)
        .unwrap();

    // rcu.clocks.timer0()
    // triggers timer at 10Hz = 0.1s
    // let timer = timer::Timer::timer0(dp.TIMER0, 10.hz(), &mut rcu);
    // let mut delay = delay::Delay::<TIMER0>::new(timer);
    let mut delay = delay::McycleDelay::new(&rcu.clocks);

    delay.delay_ms(2500_u16);
    cls!();

    // SPI1_SCK(PB13), SPI1_MISO(PB14) and SPI1_MOSI(PB15) GPIO pin configuration
    let spi = Spi::spi1(
        dp.SPI1,
        (
            gpiob.pb13.into_alternate_push_pull(),
            gpiob.pb14.into_floating_input(),
            gpiob.pb15.into_alternate_push_pull(),
        ),
        MODE_0,
        20.mhz(), // 16.mzh()
        &mut rcu,
    );

    let mut cs = gpiob.pb12.into_push_pull_output();
    cs.set_low().unwrap();

    let mut cntlr = sdmmc::Controller::new(sdmmc::SdMmcSpi::new(spi, cs), DummyTimeSource);

    buf.clear();
    match cntlr.device().init() {
        Ok(_) => {
            match cntlr.device().card_size_bytes() {
                Ok(size) => {
                    writeln!(buf, "Device OK!\nCard size: {}mb", size / 1024 / 1024).unwrap()
                }
                Err(e) => writeln!(buf, "Err: {:?}", e).unwrap(),
            }
            for i in 0..3 {
                write!(buf, "Volume {}: ", i).unwrap();
                match cntlr.get_volume(sdmmc::VolumeIdx(i)) {
                    Ok(_) => writeln!(buf, "found").unwrap(),
                    Err(_e) => {
                        writeln!(buf, "none").unwrap();
                        break;
                    }
                }
            }
        }
        Err(e) => writeln!(buf, "{:?}!", e).unwrap(),
    }
    Text::new(buf.as_str(), Point::new(0, 0))
        .into_styled(style)
        .draw(&mut lcd)
        .unwrap();
    delay.delay_ms(2_000_u16);

    cls!();
    buf.clear();

    let mut img_buf = [0u8; 106 * 80 * 2];
    // first volume
    'outer: loop {
        let mut vol = cntlr.get_volume(sdmmc::VolumeIdx(0)).unwrap();
        let dir = match cntlr.open_root_dir(&vol) {
            Ok(dir) => dir,
            Err(e) => {
                let _ = writeln!(buf, "E: {:?}", e);
                break;
            }
        };
        // read sample
        /*
        let mut fp =
            match cntlr.open_file_in_dir(&mut vol, &dir, "sample.raw", sdmmc::Mode::ReadOnly) {
                Ok(fp) => fp,
                Err(e) => {
                    let _ = writeln!(buf, "E: {:?}", e);
                    break;
                }
            };
        let nread = match cntlr.read(&mut vol, &mut fp, &mut img_buf[..]) {
            Ok(n) => n,
            Err(e) => {
                let _ = writeln!(buf, "E: {:?}", e);
                break;
            }
        };
        let _ = writeln!(buf, "Read: {}", nread);
        let _ = writeln!(buf, "bytes: {:02x}{:02x}", img_buf[0], img_buf[1]);

        drop(fp);
        */
        // Must use DOS 8.3 name
        let mut fp =
            match cntlr.open_file_in_dir(&mut vol, &dir, "badapple.raw", sdmmc::Mode::ReadOnly) {
                Ok(fp) => fp,
                Err(e) => {
                    let _ = writeln!(buf, "E: {:?}", e);
                    break;
                }
            };

        loop {
            let nread = match cntlr.read(&mut vol, &mut fp, &mut img_buf[..]) {
                Ok(n) => n,
                Err(e) => {
                    let _ = writeln!(buf, "E: {:?}", e);
                    break 'outer;
                }
            };
            if nread == 0 {
                let _ = writeln!(buf, "Done!");
                break;
            }
            let _ = fp.seek_from_current(106 * 80 * 2); // skip 1 frame

            let raw_image: ImageRaw<Rgb565> = ImageRaw::new(&img_buf[..], 106, 80);
            let image: Image<_, Rgb565> = Image::new(&raw_image, Point::new(26, 0));
            image.draw(&mut lcd).unwrap();
            delay.delay_ms(18_u16); // 1_000 / 24
        }

        break;
    }

    cls!();
    loop {
        Text::new(buf.as_str(), Point::new(0, 0))
            .into_styled(style)
            .draw(&mut lcd)
            .unwrap();
        delay.delay_ms(2_000_u16);
        cls!();
        let raw_image: ImageRaw<Rgb565> = ImageRaw::new(&img_buf[..], 106, 80);
        let image: Image<_, Rgb565> = Image::new(&raw_image, Point::new(26, 0));
        image.draw(&mut lcd).unwrap();
        delay.delay_ms(2_000_u16);
    }
}

/// Zero time as fake time source.
pub struct DummyTimeSource;

impl sdmmc::TimeSource for DummyTimeSource {
    fn get_timestamp(&self) -> sdmmc::Timestamp {
        sdmmc::Timestamp::from_fat(0, 0)
    }
}
