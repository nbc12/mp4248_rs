#![no_std]

use embedded_hal_async::i2c::I2c;

pub const DEFAULT_ADDRESS: u8 = 0x67;

#[repr(u8)]
#[derive(Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
enum Reg {
    Operation = 0x01,
    ClearFaults = 0x03,
    VoutCommand = 0x21,
    StatusWord = 0x79,
    StatusTemperature = 0x7D,
    MfrCtrl1 = 0xD0,
    MfrCurrentLimit = 0xD1,
    MfrCtrl2 = 0xD2,
    MfrCtrl3 = 0xD3,
    MfrCtrl4 = 0xD4,
    MfrStatusMask = 0xD8,
    MfrOtpConfigCode = 0xD9,
    MfrOtpRevision = 0xDA,
}

// ── Public enums ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SwitchingFrequency {
    F280kHz = 0b00,
    F420kHz = 0b01,
    F580kHz = 0b10,
}

impl SwitchingFrequency {
    fn from_bits(b: u8) -> Self {
        match b & 0b11 {
            0b00 => Self::F280kHz,
            0b10 => Self::F580kHz,
            _ => Self::F420kHz,
        }
    }
}

/// External SWA FET RDS(ON) — must match the actual FET on the board.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SwaFetRon {
    Mohm5 = 0b00,
    Mohm10 = 0b01,
    Mohm15 = 0b10,
    Mohm20 = 0b11,
}

impl SwaFetRon {
    fn from_bits(b: u8) -> Self {
        match b & 0b11 {
            0b00 => Self::Mohm5,
            0b10 => Self::Mohm15,
            0b11 => Self::Mohm20,
            _ => Self::Mohm10,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum LineDropCompensation {
    None = 0b00,
    Mv100At3A = 0b01,
    Mv300At3A = 0b10,
    Mv600At3A = 0b11,
}

impl LineDropCompensation {
    fn from_bits(b: u8) -> Self {
        match b & 0b11 {
            0b01 => Self::Mv100At3A,
            0b10 => Self::Mv300At3A,
            0b11 => Self::Mv600At3A,
            _ => Self::None,
        }
    }
}

/// SWC peak current limit / SWB valley current limit
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SwitchingCurrentLimit {
    /// 8 A SWC peak / 6 A SWB valley
    Swc8Swb6 = 0b00,
    /// 12 A SWC peak / 9 A SWB valley
    Swc12Swb9 = 0b01,
    /// 15 A SWC peak / 13 A SWB valley
    Swc15Swb13 = 0b10,
    /// 20 A SWC peak / 17 A SWB valley
    Swc20Swb17 = 0b11,
}

impl SwitchingCurrentLimit {
    fn from_bits(b: u8) -> Self {
        match b & 0b11 {
            0b00 => Self::Swc8Swb6,
            0b10 => Self::Swc15Swb13,
            0b11 => Self::Swc20Swb17,
            _ => Self::Swc12Swb9,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Rsens {
    Mohm5 = 0,
    Mohm10 = 1,
}

/// VREF rising slew rate. Multiply by fb_ratio for VOUT slew rate.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SlewRateRise {
    Mv0_08PerUs = 0b00,
    Mv0_16PerUs = 0b01,
    Mv0_40PerUs = 0b10,
    Mv0_80PerUs = 0b11,
}

impl SlewRateRise {
    fn from_bits(b: u8) -> Self {
        match b & 0b11 {
            0b00 => Self::Mv0_08PerUs,
            0b10 => Self::Mv0_40PerUs,
            0b11 => Self::Mv0_80PerUs,
            _ => Self::Mv0_16PerUs,
        }
    }
}

/// VREF falling slew rate. Multiply by fb_ratio for VOUT slew rate.
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum SlewRateFall {
    Mv0_02PerUs = 0b00,
    Mv0_04PerUs = 0b01,
    Mv0_10PerUs = 0b10,
    Mv0_20PerUs = 0b11,
}

impl SlewRateFall {
    fn from_bits(b: u8) -> Self {
        match b & 0b11 {
            0b00 => Self::Mv0_02PerUs,
            0b10 => Self::Mv0_10PerUs,
            0b11 => Self::Mv0_20PerUs,
            _ => Self::Mv0_04PerUs,
        }
    }
}

/// Blanking time before entering CC mode
#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u8)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum CcBlankTimer {
    Us250 = 0b00,
    Ms3 = 0b01,
    Ms14_5 = 0b10,
}

impl CcBlankTimer {
    fn from_bits(b: u8) -> Self {
        match b & 0b11 {
            0b01 => Self::Ms3,
            0b10 => Self::Ms14_5,
            _ => Self::Us250,
        }
    }
}

// ── Status types ──────────────────────────────────────────────────────────────

/// Decoded STATUS_WORD (0x79)
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct StatusWord(pub u16);

impl StatusWord {
    pub fn vout_ov_fault(&self) -> bool { self.0 & (1 << 5) != 0 }
    pub fn iout_oc_fault(&self) -> bool { self.0 & (1 << 4) != 0 }
    pub fn temperature_fault(&self) -> bool { self.0 & (1 << 2) != 0 }
    pub fn vout_fault(&self) -> bool { self.0 & (1 << 15) != 0 }
    pub fn iout_fault(&self) -> bool { self.0 & (1 << 14) != 0 }
    pub fn oc_exit(&self) -> bool { self.0 & (1 << 12) != 0 }
    /// True when output is within regulation (PG_STATUS# is active-low)
    pub fn power_good(&self) -> bool { self.0 & (1 << 11) == 0 }
    pub fn any_fault(&self) -> bool {
        self.vout_ov_fault() || self.iout_oc_fault() || self.temperature_fault()
    }
}

/// Decoded STATUS_TEMPERATURE (0x7D)
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct StatusTemperature(pub u8);

impl StatusTemperature {
    pub fn ot_fault(&self) -> bool { self.0 & (1 << 7) != 0 }
    pub fn ot_warning(&self) -> bool { self.0 & (1 << 6) != 0 }
    pub fn ot_warning_exit(&self) -> bool { self.0 & (1 << 3) != 0 }
}

/// Combined fault snapshot — returned by `read_alert_report()` in your alert-pin handler
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct AlertReport {
    pub status_word: StatusWord,
    pub status_temperature: StatusTemperature,
}

impl AlertReport {
    pub fn any_fault(&self) -> bool {
        self.status_word.any_fault() || self.status_temperature.ot_fault()
    }
}

// ── Charger trait ─────────────────────────────────────────────────────────────

/// Full hardware abstraction over the MP4248. Implement this on a mock for FSM unit tests.
#[allow(async_fn_in_trait)]
pub trait ChargerPSU {
    type Error;

    // OPERATION (0x01)
    async fn set_output_enabled(&mut self, enabled: bool) -> Result<(), Self::Error>;
    async fn get_output_enabled(&mut self) -> Result<bool, Self::Error>;

    // CLEAR_FAULTS (0x03)
    async fn clear_faults(&mut self) -> Result<(), Self::Error>;

    // VOUT_COMMAND (0x21)
    async fn set_vout_mv(&mut self, vout_mv: u32) -> Result<(), Self::Error>;
    async fn get_vout_mv(&mut self) -> Result<u32, Self::Error>;

    // STATUS_WORD (0x79)
    async fn read_status_word(&mut self) -> Result<StatusWord, Self::Error>;

    // STATUS_TEMPERATURE (0x7D)
    async fn read_status_temperature(&mut self) -> Result<StatusTemperature, Self::Error>;

    // Alert handling
    async fn read_alert_report(&mut self) -> Result<AlertReport, Self::Error>;

    // MFR_CTRL1 (0xD0)
    async fn set_dither_enable(&mut self, enable: bool) -> Result<(), Self::Error>;
    async fn get_dither_enable(&mut self) -> Result<bool, Self::Error>;
    async fn set_switching_frequency(&mut self, freq: SwitchingFrequency) -> Result<(), Self::Error>;
    async fn get_switching_frequency(&mut self) -> Result<SwitchingFrequency, Self::Error>;
    async fn set_swa_fet_ron(&mut self, ron: SwaFetRon) -> Result<(), Self::Error>;
    async fn get_swa_fet_ron(&mut self) -> Result<SwaFetRon, Self::Error>;
    async fn set_output_ovp_enable(&mut self, enable: bool) -> Result<(), Self::Error>;
    async fn get_output_ovp_enable(&mut self) -> Result<bool, Self::Error>;
    async fn set_output_discharge_enable(&mut self, enable: bool) -> Result<(), Self::Error>;
    async fn get_output_discharge_enable(&mut self) -> Result<bool, Self::Error>;
    async fn set_forced_pwm(&mut self, forced: bool) -> Result<(), Self::Error>;
    async fn get_forced_pwm(&mut self) -> Result<bool, Self::Error>;

    // MFR_CURRENT_LIMIT (0xD1)
    async fn set_current_limit_ma(&mut self, current_ma: u16) -> Result<(), Self::Error>;
    async fn get_current_limit_ma(&mut self) -> Result<u16, Self::Error>;
    async fn set_ldc_disable(&mut self, disable: bool) -> Result<(), Self::Error>;
    async fn get_ldc_disable(&mut self) -> Result<bool, Self::Error>;

    // MFR_CTRL2 (0xD2)
    async fn set_line_drop_compensation(&mut self, comp: LineDropCompensation) -> Result<(), Self::Error>;
    async fn get_line_drop_compensation(&mut self) -> Result<LineDropCompensation, Self::Error>;

    // MFR_CTRL3 (0xD3)
    async fn set_switching_current_limit(&mut self, lim: SwitchingCurrentLimit) -> Result<(), Self::Error>;
    async fn get_switching_current_limit(&mut self) -> Result<SwitchingCurrentLimit, Self::Error>;
    async fn set_rsens(&mut self, rsens: Rsens) -> Result<(), Self::Error>;
    async fn get_rsens(&mut self) -> Result<Rsens, Self::Error>;
    async fn set_slew_rate_rise(&mut self, rate: SlewRateRise) -> Result<(), Self::Error>;
    async fn get_slew_rate_rise(&mut self) -> Result<SlewRateRise, Self::Error>;
    async fn set_slew_rate_fall(&mut self, rate: SlewRateFall) -> Result<(), Self::Error>;
    async fn get_slew_rate_fall(&mut self) -> Result<SlewRateFall, Self::Error>;
    async fn set_freq_mode_same(&mut self, same: bool) -> Result<(), Self::Error>;
    async fn get_freq_mode_same(&mut self) -> Result<bool, Self::Error>;

    // MFR_CTRL4 (0xD4)
    async fn set_cc_blank_timer(&mut self, timer: CcBlankTimer) -> Result<(), Self::Error>;
    async fn get_cc_blank_timer(&mut self) -> Result<CcBlankTimer, Self::Error>;
    async fn set_cc_disable(&mut self, disable: bool) -> Result<(), Self::Error>;
    async fn get_cc_disable(&mut self) -> Result<bool, Self::Error>;
    async fn set_i2c_address_bits(&mut self, a5_a1: u8) -> Result<(), Self::Error>;
    async fn get_i2c_address_bits(&mut self) -> Result<u8, Self::Error>;

    // MFR_STATUS_MASK (0xD8)
    async fn read_status_mask(&mut self) -> Result<u8, Self::Error>;
    async fn write_status_mask(&mut self, value: u8) -> Result<(), Self::Error>;

    // OTP (0xD9, 0xDA)
    async fn read_otp_config_code(&mut self) -> Result<u8, Self::Error>;
    async fn read_otp_revision(&mut self) -> Result<u8, Self::Error>;
}

// ── Driver ────────────────────────────────────────────────────────────────────

#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct Mp4248<I2C> {
    i2c: I2C,
    address: u8,
    /// Feedback resistor ratio (R1+R2)/R2. Use 10 for ≤21 V, 15 for 28 V EPR mode.
    fb_ratio: u32,
}

impl<I2C, E> Mp4248<I2C>
where
    I2C: I2c<Error = E>,
{
    pub fn new(i2c: I2C, address: u8, fb_ratio: u32) -> Self {
        Self { i2c, address, fb_ratio }
    }

    pub fn release(self) -> I2C {
        self.i2c
    }

    async fn read_byte(&mut self, reg: Reg) -> Result<u8, E> {
        let mut buf = [0u8; 1];
        self.i2c.write_read(self.address, &[reg as u8], &mut buf).await?;
        Ok(buf[0])
    }

    async fn write_byte(&mut self, reg: Reg, value: u8) -> Result<(), E> {
        self.i2c.write(self.address, &[reg as u8, value]).await
    }

    async fn rmw_byte(&mut self, reg: Reg, mask: u8, bits: u8) -> Result<(), E> {
        let cur = self.read_byte(reg).await?;
        self.write_byte(reg, (cur & !mask) | (bits & mask)).await
    }

    async fn read_word_le(&mut self, reg: Reg) -> Result<u16, E> {
        let mut buf = [0u8; 2];
        self.i2c.write_read(self.address, &[reg as u8], &mut buf).await?;
        Ok(u16::from_le_bytes(buf))
    }

    async fn write_word_le(&mut self, reg: Reg, value: u16) -> Result<(), E> {
        let b = value.to_le_bytes();
        self.i2c.write(self.address, &[reg as u8, b[0], b[1]]).await
    }
}

// ── Charger impl for Mp4248 ───────────────────────────────────────────────────

impl<I2C, E> ChargerPSU for Mp4248<I2C>
where
    I2C: I2c<Error = E>,
{
    type Error = E;

    async fn set_output_enabled(&mut self, enabled: bool) -> Result<(), E> {
        self.write_byte(Reg::Operation, if enabled { 0x80 } else { 0x00 }).await
    }

    async fn get_output_enabled(&mut self) -> Result<bool, E> {
        Ok(self.read_byte(Reg::Operation).await? & 0x80 != 0)
    }

    async fn clear_faults(&mut self) -> Result<(), E> {
        self.i2c.write(self.address, &[Reg::ClearFaults as u8]).await
    }

    async fn set_vout_mv(&mut self, vout_mv: u32) -> Result<(), E> {
        let denom = self.fb_ratio * 100;
        let raw = (vout_mv * 1024 + denom - 1) / denom;
        self.write_word_le(Reg::VoutCommand, raw as u16).await
    }

    async fn get_vout_mv(&mut self) -> Result<u32, E> {
        let raw = self.read_word_le(Reg::VoutCommand).await? as u32;
        Ok(raw * self.fb_ratio * 100 / 1024)
    }

    async fn read_status_word(&mut self) -> Result<StatusWord, E> {
        Ok(StatusWord(self.read_word_le(Reg::StatusWord).await?))
    }

    async fn read_status_temperature(&mut self) -> Result<StatusTemperature, E> {
        Ok(StatusTemperature(self.read_byte(Reg::StatusTemperature).await?))
    }

    async fn read_alert_report(&mut self) -> Result<AlertReport, E> {
        Ok(AlertReport {
            status_word: self.read_status_word().await?,
            status_temperature: self.read_status_temperature().await?,
        })
    }

    async fn set_dither_enable(&mut self, enable: bool) -> Result<(), E> {
        self.rmw_byte(Reg::MfrCtrl1, 1 << 7, (enable as u8) << 7).await
    }

    async fn get_dither_enable(&mut self) -> Result<bool, E> {
        Ok(self.read_byte(Reg::MfrCtrl1).await? & (1 << 7) != 0)
    }

    async fn set_switching_frequency(&mut self, freq: SwitchingFrequency) -> Result<(), E> {
        self.rmw_byte(Reg::MfrCtrl1, 0b11 << 5, (freq as u8) << 5).await
    }

    async fn get_switching_frequency(&mut self) -> Result<SwitchingFrequency, E> {
        Ok(SwitchingFrequency::from_bits(self.read_byte(Reg::MfrCtrl1).await? >> 5))
    }

    async fn set_swa_fet_ron(&mut self, ron: SwaFetRon) -> Result<(), E> {
        self.rmw_byte(Reg::MfrCtrl1, 0b11 << 3, (ron as u8) << 3).await
    }

    async fn get_swa_fet_ron(&mut self) -> Result<SwaFetRon, E> {
        Ok(SwaFetRon::from_bits(self.read_byte(Reg::MfrCtrl1).await? >> 3))
    }

    async fn set_output_ovp_enable(&mut self, enable: bool) -> Result<(), E> {
        self.rmw_byte(Reg::MfrCtrl1, 1 << 2, (enable as u8) << 2).await
    }

    async fn get_output_ovp_enable(&mut self) -> Result<bool, E> {
        Ok(self.read_byte(Reg::MfrCtrl1).await? & (1 << 2) != 0)
    }

    async fn set_output_discharge_enable(&mut self, enable: bool) -> Result<(), E> {
        self.rmw_byte(Reg::MfrCtrl1, 1 << 1, (enable as u8) << 1).await
    }

    async fn get_output_discharge_enable(&mut self) -> Result<bool, E> {
        Ok(self.read_byte(Reg::MfrCtrl1).await? & (1 << 1) != 0)
    }

    async fn set_forced_pwm(&mut self, forced: bool) -> Result<(), E> {
        self.rmw_byte(Reg::MfrCtrl1, 1, forced as u8).await
    }

    async fn get_forced_pwm(&mut self) -> Result<bool, E> {
        Ok(self.read_byte(Reg::MfrCtrl1).await? & 1 != 0)
    }

    async fn set_current_limit_ma(&mut self, current_ma: u16) -> Result<(), E> {
        let steps = ((current_ma / 50) as u8).min(108);
        self.rmw_byte(Reg::MfrCurrentLimit, 0x7F, steps).await
    }

    async fn get_current_limit_ma(&mut self) -> Result<u16, E> {
        Ok(((self.read_byte(Reg::MfrCurrentLimit).await? & 0x7F) as u16) * 50)
    }

    async fn set_ldc_disable(&mut self, disable: bool) -> Result<(), E> {
        self.rmw_byte(Reg::MfrCurrentLimit, 1 << 7, (disable as u8) << 7).await
    }

    async fn get_ldc_disable(&mut self) -> Result<bool, E> {
        Ok(self.read_byte(Reg::MfrCurrentLimit).await? & (1 << 7) != 0)
    }

    async fn set_line_drop_compensation(&mut self, comp: LineDropCompensation) -> Result<(), E> {
        self.rmw_byte(Reg::MfrCtrl2, 0b11, comp as u8).await
    }

    async fn get_line_drop_compensation(&mut self) -> Result<LineDropCompensation, E> {
        Ok(LineDropCompensation::from_bits(self.read_byte(Reg::MfrCtrl2).await?))
    }

    async fn set_switching_current_limit(&mut self, lim: SwitchingCurrentLimit) -> Result<(), E> {
        self.rmw_byte(Reg::MfrCtrl3, 0b11 << 6, (lim as u8) << 6).await
    }

    async fn get_switching_current_limit(&mut self) -> Result<SwitchingCurrentLimit, E> {
        Ok(SwitchingCurrentLimit::from_bits(self.read_byte(Reg::MfrCtrl3).await? >> 6))
    }

    async fn set_rsens(&mut self, rsens: Rsens) -> Result<(), E> {
        self.rmw_byte(Reg::MfrCtrl3, 1 << 5, (rsens as u8) << 5).await
    }

    async fn get_rsens(&mut self) -> Result<Rsens, E> {
        Ok(if self.read_byte(Reg::MfrCtrl3).await? & (1 << 5) != 0 {
            Rsens::Mohm10
        } else {
            Rsens::Mohm5
        })
    }

    async fn set_slew_rate_rise(&mut self, rate: SlewRateRise) -> Result<(), E> {
        self.rmw_byte(Reg::MfrCtrl3, 0b11 << 3, (rate as u8) << 3).await
    }

    async fn get_slew_rate_rise(&mut self) -> Result<SlewRateRise, E> {
        Ok(SlewRateRise::from_bits(self.read_byte(Reg::MfrCtrl3).await? >> 3))
    }

    async fn set_slew_rate_fall(&mut self, rate: SlewRateFall) -> Result<(), E> {
        self.rmw_byte(Reg::MfrCtrl3, 0b11 << 1, (rate as u8) << 1).await
    }

    async fn get_slew_rate_fall(&mut self) -> Result<SlewRateFall, E> {
        Ok(SlewRateFall::from_bits(self.read_byte(Reg::MfrCtrl3).await? >> 1))
    }

    async fn set_freq_mode_same(&mut self, same: bool) -> Result<(), E> {
        self.rmw_byte(Reg::MfrCtrl3, 1, same as u8).await
    }

    async fn get_freq_mode_same(&mut self) -> Result<bool, E> {
        Ok(self.read_byte(Reg::MfrCtrl3).await? & 1 != 0)
    }

    async fn set_cc_blank_timer(&mut self, timer: CcBlankTimer) -> Result<(), E> {
        self.rmw_byte(Reg::MfrCtrl4, 0b11 << 6, (timer as u8) << 6).await
    }

    async fn get_cc_blank_timer(&mut self) -> Result<CcBlankTimer, E> {
        Ok(CcBlankTimer::from_bits(self.read_byte(Reg::MfrCtrl4).await? >> 6))
    }

    async fn set_cc_disable(&mut self, disable: bool) -> Result<(), E> {
        self.rmw_byte(Reg::MfrCtrl4, 1 << 5, (disable as u8) << 5).await
    }

    async fn get_cc_disable(&mut self) -> Result<bool, E> {
        Ok(self.read_byte(Reg::MfrCtrl4).await? & (1 << 5) != 0)
    }

    async fn set_i2c_address_bits(&mut self, a5_a1: u8) -> Result<(), E> {
        self.rmw_byte(Reg::MfrCtrl4, 0x1F, a5_a1 & 0x1F).await
    }

    async fn get_i2c_address_bits(&mut self) -> Result<u8, E> {
        Ok(self.read_byte(Reg::MfrCtrl4).await? & 0x1F)
    }

    async fn read_status_mask(&mut self) -> Result<u8, E> {
        self.read_byte(Reg::MfrStatusMask).await
    }

    async fn write_status_mask(&mut self, value: u8) -> Result<(), E> {
        self.write_byte(Reg::MfrStatusMask, value).await
    }

    async fn read_otp_config_code(&mut self) -> Result<u8, E> {
        self.read_byte(Reg::MfrOtpConfigCode).await
    }

    async fn read_otp_revision(&mut self) -> Result<u8, E> {
        self.read_byte(Reg::MfrOtpRevision).await
    }
}
