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

use gd32vf103xx_hal::gpio::{
    gpioa::PA2, gpioa::PA8, gpioc::PC13, Floating, Input, Output, PushPull,
};

static mut COUNT: i32 = 0;

static mut BTN: Option<PA8<Input<Floating>>> = None;
static mut RED: Option<PC13<Output<PushPull>>> = None;
static mut BLUE: Option<PA2<Output<PushPull>>> = None;
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
    let mut green = gpioa.pa1.into_push_pull_output();
    let mut red = gpioc.pc13.into_push_pull_output();
    let mut blue = gpioa.pa2.into_push_pull_output();
    green.set_high().unwrap();
    red.set_high().unwrap();
    blue.set_high().unwrap();
    unsafe {
        RED = Some(red);
        BLUE = Some(blue);
    };

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

    // - usb basic init
    //  /* set the host channel numbers */
    // usb_basic->num_pipe = USBFS_MAX_CHANNEL_COUNT; = 8
    // /* set the device endpoint numbers */
    // usb_basic->num_ep = USBFS_MAX_EP_COUNT; = 4
    // /* USBFS core use embedded physical layer */
    // usb_basic->phy_itf = USB_EMBEDDED_PHY;  = 2

    //  usb_basic->sof_enable = USB_SOF_OUTPUT; =  1
    // usb_basic->low_power = USB_LOW_POWER; == 1
    unsafe {
        // usb_regs.gr = USBFS_GLOBAL
        // usb_regs.hr = USBFS_HOST    0x5000_0400
        // usb_regs.dr = USBFS_DEVICE     0x5000_0800
        // usb_regs.HPCS = USBFS_HOST.hpcs
        // usb_regs.PWRCLKCTL = USBFS_PWRCLK.pwrclkctl   0x5000_0e00
        // (*pac::USBFS_GLOBAL::ptr());
    }

    // assign device endpoint registers address
    // num_ep 设置？ = 4
    // usb_regs->er_in in endpoint reg
    // USBFS_DEVICE.diep0ctl,diep1ctl,diep2ctl,diep3ctl

    // usb_regs->er_out
    // USBFS_DEVICE.doep0ctl, doep1ctl, doep2ctl, doep3ctl,

    // assign host pipe registers address
    // USB Host channel-x control register
    //usb_regs->pr   num_pipe = 8
    // USBFS_HOST.hch0ctl, hch1ctl, .... hch7ctl
    // usb_regs->DFIFO 8 个, 无文档
    // 0x5000_0000 + 0x1000 + (i* 0x1000)

    // # usb basic init ends.

    // #  usb_core_init
    // initailizes the USB core
    // initializes the USB controller registers and
    // prepares the core device mode or host mode operation

    unsafe {
        // disable USB global interrupt
        (*pac::USBFS_GLOBAL::ptr())
            .gahbcs
            .modify(|_, w| w.ginten().clear_bit());

        // phy layer (embeded) usb_basic.phy_itf
        // GUSBCS_EMBPHY bit6, undocumented
        (*pac::USBFS_GLOBAL::ptr())
            .gusbcs
            .modify(|r, w| w.bits(r.bits() | (1 << 6)));

        // soft reset the core
        //  enable core soft reset
        (*pac::USBFS_GLOBAL::ptr())
            .grstctl
            .modify(|_, w| w.csrst().set_bit());
        // wait for the core to be soft reset
        while (*pac::USBFS_GLOBAL::ptr())
            .grstctl
            .read()
            .csrst()
            .bit_is_set()
        {}
        // wait for addtional 3 PHY clocks
        delay.delay_us(3);

        // sof en
        (*pac::USBFS_GLOBAL::ptr()).gccfg.write(|w| {
            w.pwron()
                .set_bit()
                .vbusacen()
                .set_bit()
                .vbusbcen()
                .set_bit()
                .vbusig()
                .set_bit()
                .sofoen()
                .set_bit()
        });
        delay.delay_ms(20)
        //
    }
    // # end of usb_core_init

    // set device disconnect
    // usbd_disconnect only in non-otg mode
    // disconnect device for 3ms
    // usb_dev_disconnect
    unsafe {
        // soft disconnect
        (*pac::USBFS_DEVICE::ptr())
            .dctl
            .modify(|r, w| w.sd().set_bit());
    }
    delay.delay_ms(3);

    // initailizes device mode
    // usb_devcore_init
    // initialize USB core registers for device mode
    unsafe {
        // force to peripheral mode
        (*pac::USBFS_GLOBAL::ptr())
            .gusbcs
            .modify(|_, w| w.fdm().clear_bit().fhm().clear_bit());
        (*pac::USBFS_GLOBAL::ptr())
            .gusbcs
            .modify(|_, w| w.fdm().set_bit());

        // restart the Phy Clock (maybe don't need to...)
        (*pac::USBFS_PWRCLK::ptr()).pwrclkctl.write(|w| w.bits(0));

        // config periodic frame interval to default value
        const FRAME_INTERVAL_80: u8 = 0;
        (*pac::USBFS_DEVICE::ptr())
            .dcfg
            .modify(|_, w| w.eopft().bits(FRAME_INTERVAL_80).ds().bits(0));

        //make sure all FIFOs are flushed
        // usb_txfifo_flush (&udev->regs, 0x10);
        // usb_rxfifo_flush (&udev->regs);
        // TODO

        // clear all pending device interrupts
        (*pac::USBFS_DEVICE::ptr()).diepinten.write(|w| w.bits(0));
        (*pac::USBFS_DEVICE::ptr()).doepinten.write(|w| w.bits(0));
        // (*pac::USBFS_DEVICE::ptr()).daepint.write(|w| w.bits(0));
        (*pac::USBFS_DEVICE::ptr()).daepinten.write(|w| w.bits(0));
    }

    // configure all IN/OUT endpoints
    // USBFS_DEVICE.diep0ctl,diep1ctl,diep2ctl,diep3ctl
    // USBFS_DEVICE.doep0ctl, doep1ctl, doep2ctl, doep3ctl,

    unsafe {
        let USBFS_DEVICE = &(*pac::USBFS_DEVICE::ptr());

        if USBFS_DEVICE.diep0ctl.read().epen().bit_is_set() {
            USBFS_DEVICE
                .diep0ctl
                .modify(|_, w| w.epd().set_bit().snak().set_bit());
        } else {
            USBFS_DEVICE.diep0ctl.write(|w| w.bits(0));
        }

        // set IN endpoint transfer length to 0
        USBFS_DEVICE.diep0len.write(|w| w.bits(0));
        // clear all pending IN endpoint interrupts
        USBFS_DEVICE.diep0intf.write(|w| w.bits(0xff));

        if USBFS_DEVICE.doep0ctl.read().epen().bit_is_set() {
            USBFS_DEVICE.doep0ctl.modify(|r, w| {
                // w.epd().set_bit().snak().set_bit(); epd not doced
                w.bits(r.bits() | !((1 << 30) | (1 << 27)))
            })
        } else {
            USBFS_DEVICE.doep0ctl.write(|w| w.bits(0));
        }

        // set OUT endpoint transfer length to 0
        USBFS_DEVICE.doep0len.write(|w| w.bits(0));
        // clear all pending OUT endpoint interrupts
        USBFS_DEVICE.doep0intf.write(|w| w.bits(0));
    }

    // # usb_devint_enable
    unsafe {
        let USBFS_GLOBAL = &(*pac::USBFS_GLOBAL::ptr());

        //clear any pending USB OTG interrupts
        USBFS_GLOBAL.gotgintf.write(|w| w.bits(0xFFFFFFFF));

        // clear any pending interrupts
        USBFS_GLOBAL.gintf.write(|w| w.bits(0xBFFFFFFF));

        // enable the USB wakeup and suspend interrupts
        USBFS_GLOBAL
            .ginten
            .write(|w| w.wkupie().set_bit().spie().set_bit());
        // enable device_mode-related interrupts
        // FIFO mode  / or DMA mode
        // NOTE: assume FIFO mode
        USBFS_GLOBAL.ginten.modify(|_, w| {
            w.rstie()
                .set_bit()
                .enumfie()
                .set_bit()
                .iepie()
                .set_bit()
                .oepie()
                .set_bit()
                .sofie()
                .set_bit()
                .mfie()
                .set_bit()
        });

        // enable USB global interrupt
        USBFS_GLOBAL.gahbcs.modify(|_, w| w.ginten().set_bit());
    }
    // end of usb_devcore_init

    // set device connect
    // usb_dev_connect - connect device
    unsafe {
        (*pac::USBFS_DEVICE::ptr())
            .dctl
            .modify(|r, w| w.sd().set_bit());
    }
    delay.delay_ms(3);

    loop {
        buf.clear();
        write!(&mut buf, "INT {:08x}   ", unsafe { COUNT }).unwrap();
        Text::new(buf.as_str(), Point::new(0, 0))
            .into_styled(style)
            .draw(&mut lcd)
            .unwrap();
        // green.toggle().unwrap();
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
            BLUE.as_mut().unwrap().toggle().unwrap();

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
        // BLUE.as_mut().unwrap().toggle().unwrap();
       //  COUNT += 1;
    }
}

#[allow(non_snake_case)]
#[no_mangle]
fn USBFS() {
    unsafe { COUNT += 100 }

    unsafe {
        let USBFS_GLOBAL = &(*pac::USBFS_GLOBAL::ptr());
        let intr = USBFS_GLOBAL.gintf.read().bits() | USBFS_GLOBAL.ginten.read().bits();
    }

    // usbd_isr
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

    let extiline = ExtiLine::from_internal_line(InternalLine::UsbWakeup);
    Exti::clear(extiline);
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
