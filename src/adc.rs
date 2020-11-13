use core::fmt::Write;

use core::marker::PhantomData;
use embedded_hal::adc::{Channel, OneShot};

use crate::hal::rcu::Rcu;
use crate::pac::{ADC0, ADC1, RCU};

use longan_nano::sprintln;

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

#[repr(u8)]
pub enum SampleTime {
    Point_1_5 = 0,
    Point_7_5,
    Point_13_5,
    Point_28_5,
    Point_41_5,
    Point_55_5,
    Point_71_5,
    Point_239_5,
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
    pub fn adc0(adc: ADC0, _rcu: &mut Rcu) -> Self {
        let mut s = Self { rb: adc };
        let rcu = unsafe { core::mem::MaybeUninit::<RCU>::uninit().assume_init() };
        // rcu_config
        unsafe {
            // enable ADC clock
            // rcu_periph_clock_enable
            //RCU_ADC0 = RCU_REGIDX_BIT(APB2EN_REG_OFFSET, 9U),
            rcu.apb2en.modify(|_, w| w.adc0en().set_bit());
            // config ADC clock
            // RCU_CKADC_CKAPB2_DIV8           ((uint32_t)0x00000003U
            rcu.cfg0
                .modify(|_, w| w.adcpsc_1_0().bits(0x3).adcpsc_2().clear_bit());
        }
        // adc_config
        unsafe {
            // adc_deinit - reset
            rcu.apb2rst.modify(|_, w| w.adc0rst().set_bit());
            rcu.apb2rst.modify(|_, w| w.adc0rst().clear_bit());

            // adc_mode_config
            s.rb.ctl0
                .modify(|_, w| w.syncm().bits(SyncMode::Free as u8));

            // ADC scan function enable
            // scan mode enable
            s.rb.ctl0.modify(|_, w| w.sm().set_bit());
            // ADC data alignment config - LSB alignment
            s.rb.ctl1.modify(|_, w| w.dal().clear_bit());

            s.tempsensor_vrefint_enable();

            //  ADC channel length config
            // configure the length of inserted channel group
            s.rb.isq.modify(|_, w| w.il().bits(0x00));
            // length = 2, should -1
            s.rb.isq.modify(|_, w| w.il().bits(2 - 1));

            // ADC temperature sensor channel config
            s.inserted_channel_config(0, 16, SampleTime::Point_239_5);
            s.inserted_channel_config(1, 17, SampleTime::Point_239_5);

            sprintln!("ADC temperature sensor channel config - done");

            // ADC trigger config
            // adc_external_trigger_source_config
            //ADC_CTL1(adc_periph) &= ~((uint32_t)ADC_CTL1_ETSIC);
            //ADC_CTL1(adc_periph) |= (uint32_t)external_trigger_source;
            // ADC0_1_EXTTRIG_INSERTED_NONE     CTL1_ETSIC(7)
            s.rb.ctl1.modify(|_, w| w.etsic().bits(7));

            // adc_external_trigger_config
            s.rb.ctl1.modify(|_, w| w.eteic().set_bit());

            //enable ADC interface
            sprintln!("config done");

            // enable
            s.rb.ctl1.modify(|_, w| w.adcon().set_bit());
            sprintln!("adc0 enabled");

            //delay

            // ADC calibration and reset calibration
            // reset the selected ADC1 calibration registers
            s.rb.ctl1.modify(|_, w| w.rstclb().set_bit());

            while s.rb.ctl1.read().rstclb().bit_is_set() {}

            // enable ADC calibration process
            s.rb.ctl1.modify(|_, w| w.clb().set_bit());
            while s.rb.ctl1.read().clb().bit_is_set() {}

            sprintln!("cali done!");

            //  syncm - syncmode
        }

        s.enable_software_trigger();
        s
    }

    pub fn read0(&self) -> u32 {
        self.rb.idata0.read().bits()
    }
    pub fn read1(&self) -> u32 {
        self.rb.idata1.read().bits()
    }

    pub fn enable_software_trigger(&mut self) {
        // TODO:INSERTED
        self.rb.ctl1.modify(|_, w| w.swicst().set_bit());
    }
    #[inline]
    fn inserted_channel_config(&mut self, rank: u8, channel: u8, sample_time: SampleTime) {
        let inserted_length = self.rb.isq.read().il().bits();
        match inserted_length - rank {
            0 => unsafe { self.rb.isq.modify(|_, w| w.isq0().bits(channel)) },
            1 => unsafe { self.rb.isq.modify(|_, w| w.isq1().bits(channel)) },
            2 => unsafe { self.rb.isq.modify(|_, w| w.isq2().bits(channel)) },
            3 => unsafe { self.rb.isq.modify(|_, w| w.isq3().bits(channel)) },
            _ => {}
        }

        match channel {
            16 => unsafe {
                self.rb
                    .sampt0
                    .modify(|_, w| w.spt16().bits(sample_time as u8));
            },
            17 => unsafe {
                self.rb
                    .sampt0
                    .modify(|_, w| w.spt17().bits(sample_time as u8));
            },
            _ => {
                sprintln!("Hello World from UART! - not implemented");
                unimplemented!()
            }
        }
    }

    /* enable the temperature sensor and Vrefint channel */

    pub fn tempsensor_vrefint_enable(&mut self) {
        self.rb.ctl1.modify(|_, w| w.tsvren().set_bit());
    }

    pub fn tempsensor_vrefint_disable(&mut self) {
        self.rb.ctl1.modify(|_, w| w.tsvren().clear_bit());
    }
    // ADC_CTL1(ADC0) |= ADC_CTL1_TSVREN;
}
