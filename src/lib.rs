#![no_std]

pub use gd32vf103xx_hal as hal;
pub use hal::pac;

pub mod adc;
pub mod lcd;
pub mod stdout;

use core::fmt;
use core::str;

/// A fmt::Write for bytes.
pub struct ByteMutWriter<'a> {
    buf: &'a mut [u8],
    cursor: usize,
}

impl<'a> ByteMutWriter<'a> {
    pub fn new(buf: &'a mut [u8]) -> Self {
        ByteMutWriter { buf, cursor: 0 }
    }

    pub fn as_str(&self) -> &str {
        unsafe { str::from_utf8_unchecked(&self.buf[0..self.cursor]) }
    }

    #[inline]
    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    pub fn clear(&mut self) {
        self.cursor = 0;
    }

    pub fn len(&self) -> usize {
        self.cursor
    }

    pub fn empty(&self) -> bool {
        self.cursor == 0
    }

    pub fn full(&self) -> bool {
        self.capacity() == self.cursor
    }
}

impl fmt::Write for ByteMutWriter<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let cap = self.capacity();
        for (i, &b) in self.buf[self.cursor..cap]
            .iter_mut()
            .zip(s.as_bytes().iter())
        {
            *i = b;
        }
        self.cursor = usize::min(cap, self.cursor + s.as_bytes().len());
        Ok(())
    }
}
