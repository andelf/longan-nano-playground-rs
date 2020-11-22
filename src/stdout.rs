//! Stdout based on the UART hooked up to the debug connector

use core::fmt::{self, Write};
use riscv::interrupt;
use gd32vf103xx_hal::{
    serial::{Serial, Tx, self},
    gpio::{Active, gpioa::{PA10, PA9}},
    time::Bps,
    rcu::Rcu,
    afio::Afio,
    pac::USART0,
};


static mut STDOUT: Option<Tx<USART0>> = None;


/// Configures stdout
pub fn configure<X, Y>(
    uart: USART0, tx: PA9<X>, rx: PA10<Y>,
    baud_rate: Bps, afio: &mut Afio, rcu: &mut Rcu
) where X: Active, Y: Active
{
    let tx = tx.into_alternate_push_pull();
    let rx = rx.into_floating_input();
    let config = serial::Config::default().baudrate(baud_rate);
    let serial = Serial::new(uart, (tx, rx), config, afio, rcu);
    let (tx, _) = serial.split();

    interrupt::free(|_| {
        unsafe {
            STDOUT.replace(tx);
        }
    })
}


/// Writes formatted string to stdout
pub fn write_fmt(args: fmt::Arguments) {
    interrupt::free(|_| unsafe {
        if let Some(stdout) = STDOUT.as_mut() {
            let _ = stdout.write_fmt(args);
        }
    })
}


/// Macro for printing to the serial standard output
#[macro_export]
macro_rules! sprint {
    ($s:expr) => {
        $crate::stdout::write_fmt(format_args!($s))
    };
    ($($tt:tt)*) => {
        $crate::stdout::write_fmt(format_args!($($tt)*))
    };
}

/// Macro for printing to the serial standard output, with a newline.
#[macro_export]
macro_rules! sprintln {
    ($s:expr) => {
        $crate::stdout::write_fmt(format_args!(concat!($s, "\n")))
    };
    ($s:expr, $($tt:tt)*) => {
        $crate::stdout::write_fmt(format_args!(concat!($s, "\n"), $($tt)*))
    };
}