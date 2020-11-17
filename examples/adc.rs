#![no_std]
#![no_main]
#![feature(asm)]

use panic_halt as _;

use core::fmt::Write;

use embedded_hal::digital::v2::ToggleableOutputPin;
use nb::block;
use riscv_rt::entry;

use embedded_graphics::fonts::{Font6x12, Font8x16, Text};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::{primitive_style, text_style};

// gd32vf103_pac
use gd32vf103xx_hal::delay::McycleDelay;
use gd32vf103xx_hal::pac;
use gd32vf103xx_hal::prelude::*;
use gd32vf103xx_hal::timer;
// board support
use longan_nano_playground::adc::{self, Adc, Temperature, Vrefint};
use longan_nano_playground::{lcd, lcd_pins, sprintln};
use longan_nano_playground::{stdout, ByteMutWriter};

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

    let mut delay = McycleDelay::new(&rcu.clocks);

    // stdout via uart0. 115200 8N1
    stdout::configure(
        dp.USART0,
        gpioa.pa9,
        gpioa.pa10,
        115200.bps(),
        &mut afio,
        &mut rcu,
    );
    // debug requires stdout configuration.

    let a0 = adc::AnalogPin(gpioa.pa0.into_analog());
    let b0 = adc::AnalogPin(gpioa.pa1.into_analog());
    // ADC

    let mut adc = Adc::adc0(dp.ADC0, &mut rcu);
    let config = adc::config::AdcConfig::default()
        .scan(adc::config::Scan::Enabled)
        .enable_inserted_channel(Default::default());

    adc.apply_config(config);

    let vrefint = Vrefint::new().enable(&mut adc);
    let temp = Temperature::new().enable(&mut adc);

    adc.configure_inserted_channel(0, &temp, adc::config::SampleTime::Point_239_5);
    adc.configure_inserted_channel(1, &vrefint, adc::config::SampleTime::Point_239_5);
    adc.configure_inserted_channel(2, &a0, adc::config::SampleTime::Point_239_5);
    adc.configure_inserted_channel(3, &b0, adc::config::SampleTime::Point_239_5);

    let mut adc = adc.enable();
    adc.calibrate();

    //adc.enable_software_trigger();
    //adc.wait_for_conversion();

    //sprintln!("ADC READ => 0x{:08x}", adc.read0());
    //sprintln!("ADC READ => 0x{:08x}", adc.read1());
    /*

    for _ in 0..10 {
        delay.delay_ms(100);
        let temperature = (1.45 - adc.read0() as f32 * 3.3 / 4096.0) * 1000.0 / 4.1 + 25.0;
        let vref_value = adc.read1() as f32 * 3.3 / 4096.0;

        sprintln!("temp: {}C", temperature);
        sprintln!("vref: {}V", vref_value);
    }
    */

    // LCD
    let lcd_pins = lcd_pins!(gpioa, gpiob);
    let mut lcd = lcd::configure(dp.SPI0, lcd_pins, &mut afio, &mut rcu);
    let (width, height) = (lcd.size().width as i32, lcd.size().height as i32);

    // LED
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
        font = Font6x12, // Font8x16, //
        text_color = Rgb565::GREEN,
        background_color = Rgb565::BLACK
    );

    let mut buf = [0u8; 20 * 5];
    let mut buf = ByteMutWriter::new(&mut buf[..]);

    // TIMER: 0.1s
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

    loop {
        sprintln!("Hello World from UART!");

        blue.toggle().unwrap();

        adc.enable_software_trigger();
        adc.wait_for_conversion();
        adc.clear_end_of_conversion_flag();

        //    delay.delay_ms(5);

        // {(V25 – Vtemperature) / Avg_Slope} + 25
        // read???
        let raw_temp = adc.read_idata3();
        let temperature = (1.45 - (raw_temp as f32 * 3.3 / 4096.0)) * 1000.0 / 4.1 + 25.0;
        let vref_value = adc.read_idata2() as f32 * 3.3 / 4096.0;
        //let a0_val = adc.read2();

        /*
        // 单次
        adc.enable_software_trigger();
        */

        buf.clear();

        writeln!(buf, "temp: {:.2}C", temperature).unwrap();
        writeln!(buf, "Vref: {:.4}V", vref_value).unwrap();
        writeln!(buf, "data0: {}", adc.read_idata0()).unwrap();
        writeln!(buf, "data1: {}", adc.read_idata1()).unwrap();
        writeln!(buf, "data2: {}", adc.read_idata2()).unwrap();
        writeln!(buf, "data3: {}", adc.read_idata3()).unwrap();

        //write!(buf, "a0: 0x{:04x}", a0_val).unwrap();
        Text::new(buf.as_str(), Point::new(0, 0))
            .into_styled(style)
            .draw(&mut lcd)
            .unwrap();

        delay.delay_ms(500);
    }
}
