//! PA8 button.
//! red: PC13<X>, green: PA1<Y>, blue: PA2<Z>

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
use longan_nano_playground::{lcd, lcd_pins};
// gd32vf103_pac
use gd32vf103xx_hal::pac;
use gd32vf103xx_hal::prelude::*;
use gd32vf103xx_hal::timer;

use riscv_rt::entry;
#[macro_use(block)]
extern crate nb;

use embedded_hal::digital::v2::{InputPin, OutputPin, ToggleableOutputPin};

use gd32vf103xx_hal::delay::McycleDelay;

use gd32vf103xx_hal::eclic::{EclicExt, Level, LevelPriorityBits, Priority, TriggerType};
use gd32vf103xx_hal::exti::{Exti, ExtiLine, TriggerEdge};
use gd32vf103xx_hal::pac::{Interrupt, ECLIC};

use gd32vf103xx_hal::gpio::{gpioa::PA8, gpioc::PC13};
use gd32vf103xx_hal::gpio::{Floating, Input, Output, PushPull};

static mut COUNT: i32 = 0;

static mut BTN: Option<PA8<Input<Floating>>> = None;
static mut RED: Option<PC13<Output<PushPull>>> = None;

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
    let gpioc = dp.GPIOC.split(&mut rcu);

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

    let mut buf = [0u8; 20 * 5];
    let mut buf = ByteMutWriter::new(&mut buf[..]);

    // # LED / BUTTON
    let mut blue = gpioa.pa1.into_push_pull_output();
    let mut red = gpioc.pc13.into_push_pull_output();
    let boot0_btn = gpioa.pa8.into_floating_input();
    // off
    let _ = blue.set_high();
    let _ = red.set_high();

    // # Delay / Timer
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

    // IRQ
    ECLIC::reset();
    ECLIC::set_threshold_level(Level::L0);
    // Use 3 bits for level, 1 for priority
    ECLIC::set_level_priority_bits(LevelPriorityBits::L3P1);

    // eclic_irq_enable(EXTI5_9_IRQn, 1, 1);
    ECLIC::setup(
        Interrupt::EXTI_LINE9_5,
        TriggerType::Level,
        Level::L1,
        Priority::P1,
    );

    // gpio_exti_source_select(GPIO_PORT_SOURCE_GPIOA, GPIO_PIN_SOURCE_8);
    afio.extiss(boot0_btn.port(), boot0_btn.pin_number());

    // ECLIC::setup(Interrupt::TIMER0_UP, TriggerType::Level, Level::L0, Priority::P0);
    unsafe { ECLIC::unmask(Interrupt::EXTI_LINE9_5) };
    // unsafe { ECLIC::unmask(Interrupt::TIMER0_UP) };

    let mut exti = Exti::new(dp.EXTI);

    let extiline = ExtiLine::from_gpio_line(boot0_btn.pin_number()).unwrap();
    exti.listen(extiline, TriggerEdge::Both);
    Exti::clear(extiline);

    unsafe {
        RED = Some(red);
        BTN = Some(boot0_btn);
    }

    unsafe { riscv::interrupt::enable() };

    loop {
        // Clear screen
        // cls!();
        buf.clear();
        write!(&mut buf, "led blinky {}", unsafe { COUNT }).unwrap();
        Text::new(buf.as_str(), Point::new(0, 0))
            .into_styled(style)
            .draw(&mut lcd)
            .unwrap();
        // blue.toggle().unwrap();
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
            // press
            if BTN.as_ref().unwrap().is_high().unwrap() {
                COUNT += 1;
                RED.as_mut().unwrap().set_low().unwrap(); // ON
            } else {
                // release
                COUNT += 10;
                RED.as_mut().unwrap().set_high().unwrap();
            }
        }
    }
}

#[allow(non_snake_case)]
#[no_mangle]
fn DefaultHandler() {
    let code = riscv::register::mcause::read().code() & 0xFFF;
    let cause = riscv::register::mcause::Exception::from(code);

    // sprintln!("DefaultHandler [code={}, cause={:?}]", code, cause);
    unsafe {
        COUNT += 1;
    }

    loop {}

    // loop {}
}
