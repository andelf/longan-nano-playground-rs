//! Analog-to-digital converter
//!
use core::marker::PhantomData;

// NOTE: embedded_hal's Channel is not suitable for ths
use embedded_hal::adc::Channel;
use gd32vf103xx_hal::gpio::gpioa::{PA0, PA1, PA2, PA3, PA4, PA5, PA6, PA7};
use gd32vf103xx_hal::gpio::gpiob::{PB0, PB1};
use gd32vf103xx_hal::gpio::gpioc::{PC0, PC1, PC2, PC3, PC4, PC5};

use gd32vf103xx_hal::gpio::Analog;

use crate::hal::delay::McycleDelay;
use crate::hal::rcu::Rcu;
use crate::pac::{ADC0, ADC1, RCU};

use crate::sprintln;

macro_rules! adc_pins {
    ($ADC:ident, $($input:ty => $chan:expr),+ $(,)*) => {
        $(
            impl Channel<$ADC> for AnalogPin<$input> {
                type ID = u8;

                fn channel() -> u8 {
                    $chan
                }
            }
        )+
    };
}

/// Contains types related to ADC configuration
#[allow(non_camel_case_types)]
pub mod config {

    // Sequence

    #[derive(Clone, Copy, Debug, PartialEq)]
    #[allow(non_camel_case_types)]
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

    impl Default for SampleTime {
        fn default() -> Self {
            SampleTime::Point_55_5
        }
    }

    /// Clock config for the ADC
    /// Check the datasheet for the maximum speed the ADC supports
    #[derive(Debug, Clone, Copy)]
    pub enum Clock {
        /// ADC prescaler select CK_APB2/2
        Apb2_div_2 = 0,
        /// ADC prescaler select CK_APB2/4
        Apb2_div_4 = 1,
        /// ADC prescaler select CK_APB2/6
        Apb2_div_6 = 2,
        /// ADC prescaler select CK_APB2/8
        Apb2_div_8 = 3,
        /// ADC prescaler select CK_APB2/12
        Apb2_div_12 = 5,
        /// ADC prescaler select CK_APB2/16
        Apb2_div_16 = 7,
    }

    /// Resolution to sample at
    #[derive(Debug, Clone, Copy)]
    #[repr(u8)]
    pub enum Resolution {
        /// 12-bit ADC resolution
        Twelve = 0,
        /// 10-bit ADC resolution
        Ten = 1,
        /// 8-bit ADC resolution
        Eight = 2,
        /// 6-bit ADC resolution
        Six = 3,
    }

    /// Inserted group trigger source.
    #[derive(Debug, Clone, Copy)]
    pub enum RegularExternalTrigger {
        Timer0_Ch0 = 0b000,
        Timer0_Ch1 = 0b001,
        Timer0_Ch2 = 0b010,
        Timer1_Ch1 = 0b011,
        Timer2_Trgo = 0b100,
        Timer3_Ch3 = 0b101,
        Exti11 = 0b110,
        None = 0b111,
    }

    /// Inserted group trigger source.
    #[derive(Debug, Clone, Copy)]
    pub enum InsertedExternalTrigger {
        Timer2_Trgo = 0b000,
        Timer0_Ch3 = 0b001,
        Timer1_Trgo = 0b010,
        Timer1_Ch0 = 0b011,
        Timer2_Ch3 = 0b100,
        Timer3_Trgo = 0b101,
        Exti15 = 0b110,
        None = 0b111,
    }

    /// ADC data alignment
    #[derive(Debug, Clone, Copy)]
    pub enum Align {
        /// LSB alignment
        Right,
        /// MSB alignment
        Left,
    }

    impl From<Align> for bool {
        fn from(a: Align) -> bool {
            match a {
                Align::Right => false,
                Align::Left => true,
            }
        }
    }

    /// Scan enable/disable
    #[derive(Debug, Clone, Copy)]
    pub enum Scan {
        /// Scan mode disabled
        Disabled,
        /// Scan mode enabled
        Enabled,
    }
    impl From<Scan> for bool {
        fn from(s: Scan) -> bool {
            match s {
                Scan::Disabled => false,
                Scan::Enabled => true,
            }
        }
    }

    /// Continuous mode enable/disable
    #[derive(Debug, Clone, Copy)]
    pub enum Continuous {
        /// Single mode, continuous disabled
        Single,
        /// Continuous mode enabled
        Continuous,
    }
    impl From<Continuous> for bool {
        fn from(c: Continuous) -> bool {
            match c {
                Continuous::Single => false,
                Continuous::Continuous => true,
            }
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct RegularChannelGroupConfig {
        pub(crate) external_trigger: RegularExternalTrigger,
    }

    impl RegularChannelGroupConfig {
        // change the external_trigger field
        pub fn external_trigger(mut self, external_trigger: RegularExternalTrigger) -> Self {
            self.external_trigger = external_trigger;
            self
        }
    }

    impl Default for RegularChannelGroupConfig {
        fn default() -> Self {
            Self {
                external_trigger: RegularExternalTrigger::None,
            }
        }
    }

    /// Inserted channel management
    #[derive(Debug, Clone, Copy)]
    pub enum Insertion {
        /// Disabled
        Triggered,
        /// Inserted channel group convert automatically
        Auto,
    }
    impl From<Insertion> for bool {
        fn from(c: Insertion) -> bool {
            match c {
                Insertion::Triggered => false,
                Insertion::Auto => true,
            }
        }
    }

    #[derive(Debug, Clone, Copy)]
    pub struct InsertedChannelGroupConfig {
        pub(crate) external_trigger: InsertedExternalTrigger,
        pub(crate) insertion: Insertion,
    }

    impl InsertedChannelGroupConfig {
        // change the external_trigger field
        pub fn external_trigger(mut self, external_trigger: InsertedExternalTrigger) -> Self {
            self.external_trigger = external_trigger;
            self
        }
        // change the insertion field
        pub fn insertion(mut self, insertion: Insertion) -> Self {
            self.insertion = insertion;
            self
        }
    }

    impl Default for InsertedChannelGroupConfig {
        fn default() -> Self {
            Self {
                external_trigger: InsertedExternalTrigger::None,
                insertion: Insertion::Triggered,
            }
        }
    }

    // TODO: DMA for regular channel
    // TODO:

    /// Configuration for the adc.
    /// There are some additional parameters on the adc peripheral that can be
    /// added here when needed but this covers several basic usecases.
    #[derive(Debug, Clone, Copy)]
    pub struct AdcConfig {
        pub(crate) clock: Clock,
        pub(crate) resolution: Resolution,
        pub(crate) align: Align,
        pub(crate) scan: Scan,
        pub(crate) continuous: Continuous,
        pub(crate) regular_channel: Option<RegularChannelGroupConfig>,
        pub(crate) inserted_channel: Option<InsertedChannelGroupConfig>,
        pub(crate) default_sample_time: SampleTime,
    }

    impl AdcConfig {
        /// change the clock field
        pub fn clock(mut self, clock: Clock) -> Self {
            self.clock = clock;
            self
        }
        /// change the resolution field
        pub fn resolution(mut self, resolution: Resolution) -> Self {
            self.resolution = resolution;
            self
        }
        /// change the align field
        pub fn align(mut self, align: Align) -> Self {
            self.align = align;
            self
        }
        /// change the scan field
        pub fn scan(mut self, scan: Scan) -> Self {
            self.scan = scan;
            self
        }
        /// change the continuous field
        pub fn continuous(mut self, continuous: Continuous) -> Self {
            self.continuous = continuous;
            self
        }
        /// change the external_trigger field
        pub fn enable_regular_channel(mut self, cfg: RegularChannelGroupConfig) -> Self {
            self.regular_channel = Some(cfg);
            self
        }
        /// change the external_trigger field
        pub fn enable_inserted_channel(mut self, cfg: InsertedChannelGroupConfig) -> Self {
            self.inserted_channel = Some(cfg);
            self
        }
        /// change the default_sample_time field
        pub fn default_sample_time(mut self, default_sample_time: SampleTime) -> Self {
            self.default_sample_time = default_sample_time;
            self
        }
    }

    impl Default for AdcConfig {
        fn default() -> Self {
            Self {
                clock: Clock::Apb2_div_16,
                resolution: Resolution::Twelve,
                align: Align::Right,
                scan: Scan::Disabled,
                continuous: Continuous::Single,
                regular_channel: None,
                inserted_channel: None,
                default_sample_time: SampleTime::Point_55_5,
            }
        }
    }
}

/// Enabled ADC (type state)
pub struct Enabled;
/// Disabled ADC (type state)
pub struct Disabled;

pub trait ED {}
impl ED for Enabled {}
impl ED for Disabled {}

/*
pub trait Channel<ADC> {
    type ID;

    fn channel(&self) -> Self::ID;
}
*/
pub struct AnalogPin<PIN>(pub PIN);

pub struct InsertedChannel<'a, PIN> {
    rank: u8,
    pin: &'a AnalogPin<PIN>,
}

adc_pins!(ADC0,
    PA0<Analog> => 0,
    PA1<Analog> => 1,
    PA2<Analog> => 2,
    PA3<Analog> => 3,
    PA4<Analog> => 4,
    PA5<Analog> => 5,
    PA6<Analog> => 6,
    PA7<Analog> => 7,
    PB0<Analog> => 8,
    PB1<Analog> => 9,
    PC0<Analog> => 10,
    PC1<Analog> => 11,
    PC2<Analog> => 12,
    PC3<Analog> => 13,
    PC4<Analog> => 14,
    PC5<Analog> => 15,
);

/// ADC configuration
pub struct Adc<ADC, ED> {
    rb: ADC,
    config: config::AdcConfig,
    // rcu: &mut Rcu,
    _enabled: PhantomData<ED>,
}

impl Adc<ADC0, Disabled> {
    pub fn adc0(adc: ADC0, _rcu: &mut Rcu) -> Self {
        let mut adc = Self::default_from_rb(adc);

        let rcu = unsafe { core::mem::MaybeUninit::<RCU>::uninit().assume_init() };
        // TODO, use rcu.regs
        // rcu_config adc0en
        // enable ADC clock
        rcu.apb2en.modify(|_, w| w.adc0en().set_bit());

        // config ADC clock
        adc.set_clock(adc.config.clock, _rcu);

        // adc_deinit
        adc.reset(_rcu);

        unsafe {
            // adc_mode_config, for single channel, use free mode
            adc.rb
                .ctl0
                .modify(|_, w| w.syncm().bits(SyncMode::Free as u8));

            // reset inserted sequence
            adc.rb.isq.modify(|_, w| w.il().bits(0x00));
        }
        adc
    }

    /// Creates ADC with default settings
    fn default_from_rb(rb: ADC0) -> Self {
        Self {
            rb,
            config: config::AdcConfig::default(),
            _enabled: PhantomData,
        }
    }

    fn set_clock(&mut self, clock: config::Clock, _rcu: &mut Rcu) {
        use self::config::Clock::*;

        let rcu = unsafe { core::mem::MaybeUninit::<RCU>::uninit().assume_init() };
        match clock {
            Apb2_div_2 | Apb2_div_4 | Apb2_div_6 | Apb2_div_8 => unsafe {
                rcu.cfg0
                    .modify(|_, w| w.adcpsc_1_0().bits(clock as u8).adcpsc_2().clear_bit());
            },
            Apb2_div_12 | Apb2_div_16 => unsafe {
                rcu.cfg0.modify(|_, w| {
                    w.adcpsc_1_0()
                        .bits(clock as u8 >> 2)
                        .adcpsc_2()
                        .bit(clock as u8 & 0x1 == 0x1)
                });
            },
        }
    }

    /// Sets the sampling resolution
    pub fn set_resolution(&mut self, resolution: config::Resolution) {
        self.config.resolution = resolution;
    }

    /// Sets the DR register alignment to left or right
    pub fn set_align(&mut self, align: config::Align) {
        self.config.align = align;
    }

    fn configure(&mut self) {
        let config = &self.config;
        // ADC scan function enable
        unsafe {
            // resolution
            self.rb
                .ovsampctl
                .modify(|_, w| w.dres().bits(config.resolution as u8));

            // data align
            self.rb.ctl1.modify(|_, w| w.dal().bit(config.align.into()));

            // scan mode
            self.rb.ctl0.modify(|_, w| w.sm().bit(config.scan.into()));

            // continuous mode
            self.rb
                .ctl1
                .modify(|_, w| w.ctn().bit(config.continuous.into()));

            // external trigger source
            if let Some(trigger_config) = config.regular_channel {
                self.rb
                    .ctl1
                    .modify(|_, w| w.etsrc().bits(trigger_config.external_trigger as u8));
                self.rb.ctl1.modify(|_, w| w.eterc().set_bit());
            }

            if let Some(trigger_config) = config.inserted_channel {
                self.rb
                    .ctl0
                    .modify(|_, w| w.ica().bit(trigger_config.insertion.into()));
                self.rb
                    .ctl1
                    .modify(|_, w| w.etsic().bits(trigger_config.external_trigger as u8));
                self.rb.ctl1.modify(|_, w| w.eteic().set_bit());
            }

            // TODO: discontinuous
            // TODO: constraints
            // - 单次转换模式只能有一个通道 RSQ0[4:0], ISQ3[4:0]
            // - 连续转换模式只能有一个通道 RSQ0[4:0], 只能regular
            // - 规则组扫描时必须 DMA
            // - 规则组和注入组不能同时工作在间断模式，同一时刻只能有一组被设置成间断模式。
        }
    }

    /// Applies all fields in AdcConfig
    pub fn apply_config(&mut self, config: config::AdcConfig) {
        self.config = config;
    }

    pub fn configure_regular_channel<CHANNEL>(
        &mut self,
        _channel: &CHANNEL,
        _sample_time: config::SampleTime,
    ) where
        CHANNEL: Channel<ADC0, ID = u8>,
    {
        unimplemented!()
    }

    /// adc_inserted_channel_config
    pub fn configure_inserted_channel<CHANNEL>(
        &mut self,
        _channel: &CHANNEL,
        rank: u8,
        sample_time: config::SampleTime,
    ) where
        CHANNEL: Channel<ADC0, ID = u8>,
    {
        // Check the sequence is long enough
        self.rb.isq.modify(|r, w| {
            let prev = r.il().bits();
            if prev < rank {
                unsafe { w.il().bits(rank) }
            } else {
                w
            }
        });

        let channel = CHANNEL::channel();

        //Set the channel in the right sequence field
        unsafe {
            // Inserted channels are converted starting from (4 - IL[1:0] - 1),
            // if IL[1:0] length is less than 4.
            match rank {
                0 => self.rb.isq.modify(|_, w| w.isq3().bits(channel)),
                1 => self.rb.isq.modify(|_, w| w.isq2().bits(channel)),
                2 => self.rb.isq.modify(|_, w| w.isq1().bits(channel)),
                3 => self.rb.isq.modify(|_, w| w.isq0().bits(channel)),
                _ => panic!("invalid rank"),
            }

            match channel {
                10..=17 => {
                    let mask = !(0x111 << (3 * (channel - 10)));
                    self.rb.sampt0.modify(|r, w| {
                        let cleared = r.bits() & mask;
                        let masked = (sample_time as u8 as u32) << (3 * (channel - 10));
                        w.bits(cleared | masked)
                    });
                }
                0..=9 => {
                    let mask = !(0x111 << (3 * channel));
                    self.rb.sampt1.modify(|r, w| {
                        let cleared = r.bits() & mask;
                        let masked = (sample_time as u8 as u32) << (3 * channel);
                        w.bits(cleared | masked)
                    });
                }
                _ => unreachable!("invalid channel"),
            }
        }
    }

    fn reset(&mut self, _rcu: &mut Rcu) {
        let rcu = unsafe { core::mem::MaybeUninit::<RCU>::uninit().assume_init() };
        rcu.apb2rst.modify(|_, w| w.adc0rst().set_bit());
        rcu.apb2rst.modify(|_, w| w.adc0rst().clear_bit());
    }

    /// Enables the adc
    pub fn enable(mut self) -> Adc<ADC0, Enabled> {
        self.configure();

        self.rb.ctl1.modify(|_, w| w.adcon().set_bit());
        sprintln!("adc0 enabled");
        Adc {
            rb: self.rb,
            // sample_time: self.sample_time,
            config: self.config,
            _enabled: PhantomData,
        }
    }
}

impl Adc<ADC0, Enabled> {
    pub fn power_down(&mut self) {
        self.rb.ctl1.modify(|_, w| w.adcon().clear_bit());
    }

    pub fn enable_software_trigger(&mut self) {
        if self.config.regular_channel.is_some() {
            self.rb.ctl1.modify(|_, w| w.swrcst().set_bit());
        }
        if self.config.inserted_channel.is_some() {
            self.rb.ctl1.modify(|_, w| w.swicst().set_bit());
        }
    }

    pub fn wait_for_conversion(&self) {
        while self.rb.stat.read().eoc().bit_is_clear() {}

        if self.config.inserted_channel.is_some() {
            while self.rb.stat.read().eoic().bit_is_clear() {}
        }
    }

    pub fn clear_end_of_conversion_flag(&self) {
        self.rb.stat.modify(|_, w| {
            if self.config.inserted_channel.is_some() {
                w.eoc().clear_bit().eoic().clear_bit()
            } else {
                w.eoc().clear_bit()
            }
        });
    }

    fn reset_calibrate(&mut self) {
        // reset the selected ADC1 calibration registers
        self.rb.ctl1.modify(|_, w| w.rstclb().set_bit());
        while self.rb.ctl1.read().rstclb().bit_is_set() {}
    }

    /// Calibrates the ADC in single channel mode
    ///
    /// Note: The ADC must be powered
    pub fn calibrate(&mut self) {
        self.reset_calibrate();
        self.rb.ctl1.modify(|_, w| w.clb().set_bit());
        while self.rb.ctl1.read().clb().bit_is_set() {}
        sprintln!("cali done!");
    }

    pub fn read_rdata(&self) -> u16 {
        self.rb.rdata.read().rdata().bits()
    }

    pub fn read0(&self) -> u16 {
        self.rb.idata0.read().idatan().bits()
    }
    pub fn read1(&self) -> u16 {
        self.rb.idata1.read().idatan().bits()
    }
    pub fn read2(&self) -> u16 {
        self.rb.idata2.read().idatan().bits()
    }
    pub fn read3(&self) -> u16 {
        self.rb.idata3.read().idatan().bits()
    }
}

/// Internal temperature sensor
pub struct Temperature<ED> {
    _marker: PhantomData<ED>,
}

impl Temperature<Disabled> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl Temperature<Disabled> {
    // To enable Temperatuer, Vrefint is required.
    pub fn enable(&mut self, adc: &mut Adc<ADC0, Disabled>) -> Temperature<Enabled> {
        adc.rb.ctl1.modify(|_, w| w.tsvren().set_bit());
        Temperature {
            _marker: PhantomData,
        }
    }
}

impl Channel<ADC0> for Temperature<Enabled> {
    type ID = u8;

    fn channel() -> u8 {
        16
    }
}

/// Vref internal signal
// internally connected to the ADC0_CH17 input channel
pub struct Vrefint<ED> {
    _marker: PhantomData<ED>,
}

impl Vrefint<Disabled> {
    pub fn new() -> Self {
        Self {
            _marker: PhantomData,
        }
    }
}

impl Vrefint<Disabled> {
    pub fn enable(self, adc: &mut Adc<ADC0, Disabled>) -> Vrefint<Enabled> {
        adc.rb.ctl1.modify(|_, w| w.tsvren().set_bit());
        Vrefint {
            _marker: PhantomData,
        }
    }
}

impl Channel<ADC0> for Vrefint<Enabled> {
    type ID = u8;

    fn channel() -> u8 {
        17
    }
}

/// ADC sync mode
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

/// Init ADC0 and ADC1 at once, enabling sync mode.
pub fn adc01(
    _adc0: ADC0,
    _adc1: ADC1,
    _sync_mode: SyncMode,
    _delay: &mut McycleDelay,
    //prec: rec::Adc12,
    //clocks: &CoreClocks,
) -> (Adc<ADC0, Disabled>, Adc<ADC1, Disabled>) {
    unimplemented!("DMA required to use sync mode")
}
