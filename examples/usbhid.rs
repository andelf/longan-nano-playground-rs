#![no_std]
#![no_main]
#![feature(asm)]

use panic_halt as _;

use riscv_rt::entry;

use core::fmt::Write;

use gd32vf103xx_hal::delay::McycleDelay;
use gd32vf103xx_hal::pac;
use gd32vf103xx_hal::prelude::*;

use gd32vf103xx_hal::eclic::{EclicExt, Level, LevelPriorityBits, Priority, TriggerType};
use gd32vf103xx_hal::exti::{Exti, ExtiLine, InternalLine, TriggerEdge};
use gd32vf103xx_hal::pac::{Interrupt, ECLIC};
use gd32vf103xx_hal::timer;

use embedded_hal::digital::v2::{InputPin, OutputPin, ToggleableOutputPin};

// LCD
use embedded_graphics::fonts::{Font8x16, Text};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::{primitive_style, text_style};
use longan_nano_playground::{lcd, lcd_pins, ByteMutWriter};

use gd32vf103xx_hal::gpio::{gpioa::PA8, gpioc::PC13, Floating, Input, Output, PushPull};

static mut COUNT: i32 = 0;

static mut BTN: Option<PA8<Input<Floating>>> = None;
static mut RED: Option<PC13<Output<PushPull>>> = None;
static mut TIMER: Option<timer::Timer<pac::TIMER2>> = None;

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Configure clocks
    // 48MHz, 72MHz, 96MHz for USB
    let mut rcu = dp
        .RCU
        .configure()
        .ext_hf_clock(8.mhz())
        .sysclk(96.mhz())
        .freeze();
    assert!(rcu.clocks.usbclk_valid());
    let mut afio = dp.AFIO.constrain(&mut rcu);

    let gpioa = dp.GPIOA.split(&mut rcu);
    let gpiob = dp.GPIOB.split(&mut rcu);
    let gpioc = dp.GPIOC.split(&mut rcu);

    let mut exti = Exti::new(dp.EXTI);

    let boot0_btn = gpioa.pa8.into_floating_input();

    let usb_dp = gpioa.pa12;
    let usb_dm = gpioa.pa11;

    let mut delay = McycleDelay::new(&rcu.clocks);

    // # LCD
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

    Text::new("USB-HID example", Point::new(0, 0))
        .into_styled(style)
        .draw(&mut lcd)
        .unwrap();
    delay.delay_ms(1_000);

    let mut buf = [0u8; 20 * 5];
    let mut buf = ByteMutWriter::new(&mut buf[..]);
    // LEDs
    let mut blue = gpioa.pa1.into_push_pull_output();
    let mut red = gpioc.pc13.into_push_pull_output();
    blue.set_high().unwrap();
    red.set_high().unwrap();
    unsafe { RED = Some(red) };

    // interrupt
    ECLIC::reset();
    ECLIC::set_threshold_level(Level::L0);
    unsafe { riscv::interrupt::enable() };
    ECLIC::set_level_priority_bits(LevelPriorityBits::L2P2);

    unsafe {
        (*pac::RCU::ptr())
            .ahben
            .modify(|_, w| w.usbfsen().set_bit());
    }

    // usb_timer_init
    let mut timer2 = timer::Timer::timer2(dp.TIMER2, 1.hz(), &mut rcu);
    ECLIC::setup(
        Interrupt::TIMER2,
        TriggerType::Level,
        Level::L3,
        Priority::P0,
    );
    unsafe { ECLIC::unmask(Interrupt::TIMER2) };
    timer2.listen(timer::Event::Update);
    unsafe {
        TIMER = Some(timer2);
    }

    Text::new("DEBUG 1", Point::new(0, 20))
        .into_styled(style)
        .draw(&mut lcd)
        .unwrap();

    //systick_config
    unsafe {
        // mtimecmp
        let mtimecmp = (rcu.clocks.sysclk().0 / 4 / 100);
        (*pac::CTIMER::ptr())
            .mtimecmp_lo
            .write(|w| w.bits(mtimecmp));
        (*pac::CTIMER::ptr()).mtimecmp_hi.write(|w| w.bits(0));
    }
    ECLIC::set_level_priority_bits(LevelPriorityBits::L2P2);
    ECLIC::setup(
        Interrupt::INT_TMR,
        TriggerType::Level,
        Level::L3,
        Priority::P3,
    );
    unsafe { ECLIC::unmask(Interrupt::INT_TMR) };
    // clear mtime
    unsafe {
        // mtime
        (*pac::CTIMER::ptr()).mtime_lo.write(|w| w.bits(0));
        (*pac::CTIMER::ptr()).mtime_hi.write(|w| w.bits(0));
    }

    // usb_intr_config
    ECLIC::setup(
        Interrupt::USBFS,
        TriggerType::Level,
        Level::L1,
        Priority::P0,
    );
    unsafe { ECLIC::unmask(Interrupt::USBFS) };

    // enable PMU
    buf.clear();
    writeln!(buf, "pmuen={}", unsafe {
        (*pac::RCU::ptr()).apb1en.read().pmuen().bits()
    });
    Text::new(buf.as_str(), Point::new(0, 40))
        .into_styled(style)
        .draw(&mut lcd)
        .unwrap();
    unsafe {
        (*pac::RCU::ptr()).apb1en.modify(|_, w| w.pmuen().set_bit());
    }
    /// EXTI_18
    let extiline = ExtiLine::from_internal_line(InternalLine::UsbWakeup);
    Exti::clear(extiline);
    exti.listen(extiline, TriggerEdge::Rising);

    ECLIC::setup(
        Interrupt::USBFS_WKUP,
        TriggerType::Level,
        Level::L3,
        Priority::P0,
    );
    unsafe { ECLIC::unmask(Interrupt::USBFS_WKUP) };

    // button BOOT0
    afio.extiss(boot0_btn.port(), boot0_btn.pin_number());
    ECLIC::setup(
        Interrupt::EXTI_LINE9_5,
        TriggerType::Level,
        Level::L1,
        Priority::P1,
    );
    let extiline = ExtiLine::from_gpio_line(boot0_btn.pin_number()).unwrap();
    unsafe { ECLIC::unmask(Interrupt::EXTI_LINE9_5) };
    exti.listen(extiline, TriggerEdge::Both);
    Exti::clear(extiline);

    unsafe {
        BTN = Some(boot0_btn);
    }

    // unsafe { riscv::interrupt::enable() };

    // usbd_init

    loop {
        buf.clear();
        write!(&mut buf, "INT {:08x}   ", unsafe { COUNT }).unwrap();
        Text::new(buf.as_str(), Point::new(0, 0))
            .into_styled(style)
            .draw(&mut lcd)
            .unwrap();
        blue.toggle().unwrap();
        delay.delay_ms(200);
    }
}

#[allow(non_snake_case)]
#[no_mangle]
fn EXTI_LINE9_5() {
    let extiline = ExtiLine::from_gpio_line(8).unwrap();
    // exti.listen(extiline, TriggerEdge::Both);
    if Exti::is_pending(extiline) {
        Exti::unpend(extiline);
        Exti::clear(extiline);

        unsafe {
            if BTN.as_ref().unwrap().is_high().unwrap() {
                COUNT += 1;
            } else {
                // release
                COUNT -= 1;
            }
        }
    }
}

#[allow(non_snake_case)]
#[no_mangle]
fn INT_TMR() {
    // get usb data here?
    unsafe {
        // mtime
        (*pac::CTIMER::ptr()).mtime_lo.write(|w| w.bits(0));
        (*pac::CTIMER::ptr()).mtime_hi.write(|w| w.bits(0));
    }

    unsafe {
        // COUNT += 1;
    }
}

#[allow(non_snake_case)]
#[no_mangle]
fn USBFS() {
    unsafe { COUNT += 100 }
}

#[allow(non_snake_case)]
#[no_mangle]
fn EXTI4() {
    unsafe { COUNT += 0x100 }
}

#[allow(non_snake_case)]
#[no_mangle]
fn USBFS_WKUP() {
    unsafe { COUNT += 0x10000 }
}

#[allow(non_snake_case)]
#[no_mangle]
fn TIMER2() {
    unsafe {
        if let Some(timer2) = &mut TIMER {
            timer2.clear_update_interrupt_flag();
        }
    }
    unsafe {
        if let Some(led) = &mut RED {
            led.toggle().unwrap();
        }
    }
    unsafe { COUNT += 0x1000000 }
}

#[allow(non_snake_case)]
#[no_mangle]
fn DefaultHandler() {
    let code = riscv::register::mcause::read().code() & 0xFFF;
    let cause = riscv::register::mcause::Exception::from(code);

    // sprintln!("DefaultHandler [code={}, cause={:?}]", code, cause);
    unsafe {
        COUNT += 0xffffff;
    }

    // loop {}

    loop {}
}
