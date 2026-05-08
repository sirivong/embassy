//! Shared module for the i2c-loopback-* examples.
//!
//! Exposes:
//!  * [`target_task`] — a target-side loop with the same semantics as
//!    `i2c-target-async.rs` (32-byte buffer, replies 0x55 to reads,
//!    accepts up to 32 bytes on writes). Address 0x2A.
//!  * [`harness::run`]   — runs the controller-side test suite against
//!    the in-chip target. Logs PASS/FAIL via defmt and panics on failure
//!    so probe-rs returns a non-zero exit.
//!
//! The two binaries (`i2c-loopback-async`, `i2c-loopback-dma`) construct
//! the controller `I2c` differently and pass it to `harness::run`.
//!
//! Wiring (one-time): jumper P0_20 ↔ P3_20 (SDA) and P0_21 ↔ P3_21 (SCL),
//! and put 4.7k pull-ups on both nets.

use embassy_mcxa as hal;

use hal::Peri;
use hal::i2c::controller::IOError as ControllerIOError;
use hal::i2c::target::{self, Address, Config as TargetConfig, InterruptHandler, Request};
use hal::interrupt::typelevel::Binding;
use hal::peripherals::{LPI2C3, P3_20, P3_21};

const TARGET_ADDR: u16 = 0x2A;
const TARGET_BUF_LEN: usize = 32;

/// Target task. Mirrors `i2c-target-async.rs` byte-for-byte.
///
/// Pinned to LPI2C3 / P3_21 SCL / P3_20 SDA (matches the `i2c-target-async`
/// example) since the i2c pin traits are not implemented for `AnyPin`.
pub async fn target_task(
    peri: Peri<'static, LPI2C3>,
    scl: Peri<'static, P3_21>,
    sda: Peri<'static, P3_20>,
    irq: impl Binding<<LPI2C3 as hal::i2c::Instance>::Interrupt, InterruptHandler<LPI2C3>> + 'static,
) -> ! {
    let mut config = TargetConfig::default();
    config.address = Address::Single(TARGET_ADDR);

    let mut tgt = target::I2c::new_async(peri, scl, sda, irq, config).unwrap();
    let mut buf = [0u8; TARGET_BUF_LEN];

    loop {
        let request = tgt.async_listen().await.unwrap();
        defmt::trace!("[T] event {}", request);
        match request {
            Request::Read(addr) => {
                buf.fill(0x55);
                let count = tgt.async_respond_to_read(&buf).await.unwrap();
                defmt::trace!("[T R {:02x}] -> {:02x}", addr, buf[..count]);
            }
            Request::Write(addr) => {
                let count = tgt.async_respond_to_write(&mut buf).await.unwrap();
                defmt::trace!("[T W {:02x}] <- {:02x}", addr, buf[..count]);
            }
            _ => {}
        }
    }
}

/// Trait abstracting an async I2C controller for the test suite.
///
/// Both `I2c<'_, Async>` and `I2c<'_, Dma<'_>>` implement
/// `embedded_hal_async::i2c::I2c`, so we use that. We need
/// `Error = IOError` for nicer messages.
pub trait Controller: embedded_hal_async::i2c::I2c<Error = ControllerIOError> {}
impl<T: embedded_hal_async::i2c::I2c<Error = ControllerIOError>> Controller for T {}

pub mod harness {
    use super::*;

    /// Run the full test suite. Logs `[NAME] PASS` per test. Panics on
    /// the first failure.
    pub async fn run<C: Controller>(ctrl: &mut C) {
        defmt::info!("== loopback test suite start ==");

        match tests::t_basic_rw(ctrl).await {
            Ok(()) => defmt::info!("[t_basic_rw] PASS"),
            Err(e) => {
                defmt::error!("[t_basic_rw] FAIL: {}", e);
                panic!("test failure");
            }
        }

        defmt::info!("== loopback test suite ALL PASS ==");
    }
}

pub mod tests {
    use super::*;

    pub const ADDR: u8 = 0x2A;
    pub const EXPECT: u8 = 0x55;

    /// 100 iters of {write 2 bytes, read 2 bytes (== 0x55,0x55)}.
    pub async fn t_basic_rw<C: Controller>(ctrl: &mut C) -> Result<(), &'static str> {
        for i in 0..100u16 {
            ctrl.write(ADDR, &[i as u8, (i >> 8) as u8])
                .await
                .map_err(|_| "write failed")?;
            let mut r = [0u8; 2];
            ctrl.read(ADDR, &mut r).await.map_err(|_| "read failed")?;
            if r != [EXPECT, EXPECT] {
                defmt::error!("iter {}: read mismatch got={:02x}", i, r);
                return Err("read mismatch");
            }
        }
        Ok(())
    }
}
