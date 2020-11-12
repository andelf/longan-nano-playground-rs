use core::marker::PhantomData;
use embedded_hal::adc::{Channel, OneShot};

use crate::hal::rcu::Rcu;
use crate::pac::{ADC0, ADC1};

#[repr(u8)]
pub enum SyncMode {
    /// all the ADCs work independently
    Free = 0,
    /// ADC0 and ADC1 work in combined regular parallel + inserted parallel mode
    DualRegulalParallelInsertedParallel,
    ///  ADC0 and ADC1 work in combined regular parallel + trigger rotation mode
    DualRegulalParallelInsertedRotation,
    // ADC0 and ADC1 work in combined inserted parallel + follow-up fast mode
    DualInsertedParallelRegulalFollowupFast,
    /// ADC0 and ADC1 work in combined inserted parallel + follow-up slow mode
    DualInsertedParallelRegulalFollowupSlow,
    /// ADC0 and ADC1 work in inserted parallel mode only
    DualInsertedParallel,
    /// ADC0 and ADC1 work in regular parallel mode only
    DualRegulalParallel,
    /// ADC0 and ADC1 work in follow-up fast mode only
    DualRegulalFollowupFast,
    /// ADC0 and ADC1 work in follow-up slow mode only
    DualRegulalFollowupSlow,
    /// ADC0 and ADC1 work in trigger rotation mode only
    DualInsertedTriggerRotation,
}

/// ADC configuration
pub struct Adc<ADC> {
    rb: ADC,
    //sample_time: SampleTime,
    //align: Align,
    //clocks: Clocks,
    // rcu: &mut Rcu,
}

impl Adc<ADC0> {
    pub fn adc0(adc: ADC0, rcu: &mut Rcu) -> Self {
        let s = Self { rb: adc };

        //  syncm - syncmode
        unsafe {
            s.rb.ctl1.modify(|_, w| a.adcon().set_bit());

            s.rb.ctl0
                .modify(|_, w| w.syncm().bits(SyncMode::Free as u8));
        }
        s
    }
}
