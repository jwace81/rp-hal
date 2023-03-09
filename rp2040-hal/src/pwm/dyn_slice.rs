//! Semi-internal enums mostly used in typelevel magic

use embedded_hal::PwmPin;

use super::{reg::RegisterInterface, Slice, SliceId, SliceMode, ValidSliceMode, Channel, ChannelId};
use crate::{atomic_register_access::{write_bitmask_clear, write_bitmask_set}, gpio::DynPin};

/// Value-level `struct` representing slice IDs
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct DynSliceId {
    /// Slice id
    pub num: u8,
}

/// Slice modes
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum DynSliceMode {
    /// Count continuously whenever the slice is enabled
    FreeRunning,
    /// Count continuously when a high level is detected on the B pin
    InputHighRunning,
    /// Count once with each rising edge detected on the B pin
    CountRisingEdge,
    /// Count once with each falling edge detected on the B pin
    CountFallingEdge,
}

/// Channel ids
#[derive(PartialEq, Eq, Clone, Copy)]
pub enum DynChannelId {
    /// Channel A
    A,
    /// Channel B
    B,
}

struct DynSliceRegisters {
    id: DynSliceId,
}

unsafe impl RegisterInterface for DynSliceRegisters {
    #[inline]
    fn id(&self) -> DynSliceId {
        self.id
    }
}

impl DynSliceRegisters {
    #[inline]
    unsafe fn new(id: DynSliceId) -> Self {
        DynSliceRegisters { id }
    }
}

pub struct DynSlice {
    regs: DynSliceRegisters,
    mode: DynSliceMode,
}

impl DynSlice {
    #[inline]
    pub unsafe fn new(
        id: DynSliceId,
        mode: DynSliceMode,
    ) -> Self {
        DynSlice {
            regs: DynSliceRegisters::new(id),
            mode,
        }
    }

    #[inline]
    pub fn id(&self) -> DynSliceId {
        self.regs.id
    }

    /// Return a copy of the pin mode
    #[inline]
    pub fn mode(&self) -> DynSliceMode {
        self.mode
    }

    /// Set a default config for the slice
    pub fn default_config(&mut self) {
        self.regs.write_ph_correct(false);
        self.regs.write_div_int(1); // No divisor
        self.regs.write_div_frac(0); // No divisor
        self.regs.write_inv_a(false); //Don't invert the channel
        self.regs.write_inv_b(false); //Don't invert the channel
        self.regs.write_top(0xffff); // Wrap at max
        self.regs.write_ctr(0x0000); //Reset the counter
        self.regs.write_cc_a(0); //Default duty cycle of 0%
        self.regs.write_cc_b(0); //Default duty cycle of 0%
    }

    /// Advance the phase with one count
    ///
    /// Counter must be running at less than full speed (div_int + div_frac / 16 > 1)
    #[inline]
    pub fn advance_phase(&mut self) {
        self.regs.advance_phase()
    }

    /// Retard the phase with one count
    ///
    /// Counter must be running at less than full speed (div_int + div_frac / 16 > 1)
    #[inline]
    pub fn retard_phase(&mut self) {
        self.regs.retard_phase()
    }

    /// Enable phase correct mode
    #[inline]
    pub fn set_ph_correct(&mut self) {
        self.regs.write_ph_correct(true)
    }

    /// Disables phase correct mode
    #[inline]
    pub fn clr_ph_correct(&mut self) {
        self.regs.write_ph_correct(false)
    }

    /// Enable slice
    #[inline]
    pub fn enable(&mut self) {
        self.regs.write_enable(true);
    }

    /// Disable slice
    #[inline]
    pub fn disable(&mut self) {
        self.regs.write_enable(false)
    }

    /// Sets the integer part of the clock divider
    #[inline]
    pub fn set_div_int(&mut self, value: u8) {
        self.regs.write_div_int(value)
    }

    /// Sets the fractional part of the clock divider
    #[inline]
    pub fn set_div_frac(&mut self, value: u8) {
        self.regs.write_div_frac(value)
    }

    /// Get the counter register value
    #[inline]
    pub fn get_counter(&self) -> u16 {
        self.regs.read_ctr()
    }

    /// Set the counter register value
    #[inline]
    pub fn set_counter(&mut self, value: u16) {
        self.regs.write_ctr(value)
    }

    /// Get the top register value
    #[inline]
    pub fn get_top(&self) -> u16 {
        self.regs.read_top()
    }

    /// Sets the top register value
    #[inline]
    pub fn set_top(&mut self, value: u16) {
        self.regs.write_top(value)
    }

    /// Create the interrupt bitmask corresponding to this slice
    #[inline]
    fn bitmask(&self) -> u32 {
        1 << self.id().num
    }

    /// Enable the PWM_IRQ_WRAP interrupt when this slice overflows.
    #[inline]
    pub fn enable_interrupt(&mut self) {
        unsafe {
            let pwm = &(*pac::PWM::ptr());
            let reg = pwm.inte.as_ptr();
            write_bitmask_set(reg, self.bitmask());
        }
    }

    /// Disable the PWM_IRQ_WRAP interrupt for this slice.
    #[inline]
    pub fn disable_interrupt(&mut self) {
        unsafe {
            let pwm = &(*pac::PWM::ptr());
            let reg = pwm.inte.as_ptr();
            write_bitmask_clear(reg, self.bitmask());
        };
    }

    /// Did this slice trigger an overflow interrupt?
    #[inline]
    pub fn has_overflown(&self) -> bool {
        let mask = self.bitmask();
        unsafe { (*pac::PWM::ptr()).ints.read().bits() & mask == mask }
    }

    /// Mark the interrupt handled for this slice.
    #[inline]
    pub fn clear_interrupt(&mut self) {
        unsafe { (*pac::PWM::ptr()).intr.write(|w| w.bits(self.bitmask())) };
    }

    /// Force the interrupt. This bit is not cleared by hardware and must be manually cleared to
    /// stop the interrupt from continuing to be asserted.
    #[inline]
    pub fn force_interrupt(&mut self) {
        unsafe {
            let pwm = &(*pac::PWM::ptr());
            let reg = pwm.intf.as_ptr();
            write_bitmask_set(reg, self.bitmask());
        }
    }

    /// Clear force interrupt. This bit is not cleared by hardware and must be manually cleared to
    /// stop the interrupt from continuing to be asserted.
    #[inline]
    pub fn clear_force_interrupt(&mut self) {
        unsafe {
            let pwm = &(*pac::PWM::ptr());
            let reg = pwm.intf.as_ptr();
            write_bitmask_clear(reg, self.bitmask());
        }
    }
}

impl<I, M> From<Slice<I, M>> for DynSlice
where
    I: SliceId,
    M: SliceMode + ValidSliceMode<I>,
{
    #[inline]
    fn from(_slice: Slice<I, M>) -> Self {
        unsafe { DynSlice::new(I::DYN, M::DYN) }
    }
}

pub struct DynChannel {
    regs: DynSliceRegisters,
    mode: DynSliceMode,
    channel_id: DynChannelId,
    duty_cycle: u16,
    enabled: bool,
}

impl DynChannel {
    #[inline]
    unsafe fn new(id: DynSliceId, mode: DynSliceMode, channel_id: DynChannelId) -> Self {
        DynChannel {
            regs: DynSliceRegisters::new(id),
            mode,
            channel_id,
            duty_cycle: 0u16,
            enabled: false,
        }
    }

    #[inline]
    pub fn slice_id(&self) -> DynSliceId {
        self.regs.id
    }

    /// Return a copy of the pin mode
    #[inline]
    pub fn mode(&self) -> DynSliceMode {
        self.mode
    }

    #[inline]
    pub fn id(&self) -> DynChannelId {
        self.channel_id
    }

    /// Invert channel output
    #[inline]
    pub fn set_inverted(&mut self) {
        match self.channel_id {
            DynChannelId::A => {
                self.regs.write_inv_a(true)
            },
            DynChannelId::B => {
                self.regs.write_inv_b(true)
            },
        }
    }

    /// Stop inverting channel output
    #[inline]
    pub fn clr_inverted(&mut self) {
        match self.channel_id {
            DynChannelId::A => {
                self.regs.write_inv_a(false)
            },
            DynChannelId::B => {
                self.regs.write_inv_b(false)
            },
        }
    }

    pub fn input_from(
        &mut self,
        mut pin: DynPin,
    ) {
        pin.try_into_mode(crate::gpio::DYN_FUNCTION_PWM).unwrap();
    }

    pub fn output_to(
        &mut self,
        mut pin: DynPin,
    ) {
        pin.try_into_mode(crate::gpio::DYN_FUNCTION_PWM).unwrap();
    }
}

impl PwmPin for DynChannel {
    type Duty = u16;

    fn disable(&mut self) {
        match self.channel_id {
            DynChannelId::A => {
                if self.enabled {
                    self.duty_cycle = self.regs.read_cc_a();
                }
                self.enabled = false;
                self.regs.write_cc_a(0)
            },
            DynChannelId::B => {
                if self.enabled {
                    self.duty_cycle = self.regs.read_cc_b();
                }
                self.enabled = false;
                self.regs.write_cc_b(0)
            }
        }
    }

    fn enable(&mut self) {
        if !self.enabled {
            self.enabled = true;

            match self.channel_id {
                DynChannelId::A => {
                    self.regs.write_cc_a(self.duty_cycle)
                },
                DynChannelId::B => {
                    self.regs.write_cc_b(self.duty_cycle)
                }
            }
        }
    }

    fn get_duty(&self) -> Self::Duty {
        if self.enabled {
            match self.channel_id {
                DynChannelId::A => {
                    self.regs.read_cc_a()
                },
                DynChannelId::B => {
                    self.regs.read_cc_b()
                }
            }
        } else {
            self.duty_cycle
        }
    }

    fn get_max_duty(&self) -> Self::Duty {
        self.regs.read_top()
    }

    fn set_duty(&mut self, duty: Self::Duty) {
        self.duty_cycle = duty;
        if self.enabled {
            match self.channel_id {
                DynChannelId::A => {
                    self.regs.write_cc_a(duty)
                },
                DynChannelId::B => {
                    self.regs.write_cc_b(duty)
                }
            }
        }
    }
}

impl<I, M, C> From<Channel<I, M, C>> for DynChannel
where
    I: SliceId,
    M: SliceMode + ValidSliceMode<I>,
    C: ChannelId,
{
    #[inline]
    fn from(_pin: Channel<I, M, C>) -> Self {
        unsafe { DynChannel::new(I::DYN, M::DYN, C::DYN) }
    }
}
