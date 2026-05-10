//! DLYB tuning helpers used by SDMMC for SDR50 / SDR104 sampling.
//! Lock-mode procedure per RM0486 §31.4.4.

use embassy_hal_internal::PeripheralType;

use super::{Error, Instance};

const POLL_LIMIT: u32 = 100_000;

#[allow(private_bounds)]
pub trait DlybInstance<T: Instance>: SealedDlybInstance<T> + PeripheralType + 'static {}
pub(crate) trait SealedDlybInstance<T: Instance> {
    fn regs() -> crate::pac::dlybsd::Dlybsd;
    fn release_dll_reset();
}

use crate::peripherals::{DLYB_SDMMC1, DLYB_SDMMC2, SDMMC1, SDMMC2};

impl SealedDlybInstance<SDMMC1> for DLYB_SDMMC1 {
    fn regs() -> crate::pac::dlybsd::Dlybsd {
        crate::pac::DLYB_SDMMC1
    }
    fn release_dll_reset() {
        crate::pac::RCC.miscrstr().modify(|w| w.set_sdmmc1dllrst(false));
    }
}
impl DlybInstance<SDMMC1> for DLYB_SDMMC1 {}

impl SealedDlybInstance<SDMMC2> for DLYB_SDMMC2 {
    fn regs() -> crate::pac::dlybsd::Dlybsd {
        crate::pac::DLYB_SDMMC2
    }
    fn release_dll_reset() {
        crate::pac::RCC.miscrstr().modify(|w| w.set_sdmmc2dllrst(false));
    }
}
impl DlybInstance<SDMMC2> for DLYB_SDMMC2 {}

pub(crate) struct Dlyb {
    regs: crate::pac::dlybsd::Dlybsd,
}

impl Dlyb {
    pub(crate) fn new(regs: crate::pac::dlybsd::Dlybsd) -> Self {
        Self { regs }
    }

    pub(crate) fn enable_lock(&mut self) -> Result<(), Error> {
        self.regs.cfg().modify(|w| {
            w.set_sdmmc_dll_byp_en(false);
            w.set_sdmmc_dll_en(true);
        });
        for _ in 0..POLL_LIMIT {
            if self.regs.status().read().sdmmc_dll_lock() {
                return Ok(());
            }
        }
        Err(Error::Timeout)
    }

    pub(crate) fn disable(&mut self) {
        self.regs.cfg().modify(|w| w.set_sdmmc_dll_en(false));
    }

    pub(crate) fn set_tap(&mut self, tap: u8) -> Result<(), Error> {
        self.regs.cfg().modify(|w| w.set_sdmmc_rx_tap_sel(tap));
        for _ in 0..POLL_LIMIT {
            if self.regs.status().read().sdmmc_rx_tap_sel_ack() {
                return Ok(());
            }
        }
        Err(Error::Timeout)
    }
}
