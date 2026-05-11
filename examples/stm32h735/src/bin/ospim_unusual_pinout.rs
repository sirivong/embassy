#![no_main]
#![no_std]

// Tested on weact stm32h7b0 board + w25q64 spi flash

use defmt::info;
use embassy_executor::Spawner;
use embassy_stm32::gpio::{Level, Output, Speed};
use embassy_stm32::mode::Blocking;
use embassy_stm32::ospi::{
    AddressSize, ChipSelectHighTime, FIFOThresholdLevel, Instance, MemorySize, MemoryType, Ospi,
    OspiWidth, TransferConfig, WrapSize,
};
use embassy_time::Timer;
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("START");

    // Initialize peripherals & RCC.
    let p = rcc_setup::stm32h735g_init();

    // Output pin PA8 (also MCO)
    let mut led = Output::new(p.PA8, Level::Low, Speed::Low);
    led.set_high();

    // on the test board, there is an FPGA connected to the OCTOSPI data lines so that the FPGA
    // can boot from the flash, with an unprogrammed FPGA it will have weak pull-ups on every IO pin
    // and these need to be disabled before the flash can be communicated with.

    info!("Hold FPGA in reset");
    // hold FPGA in RESET mode
    let mut fpga_creset_b = Output::new(p.PF15, Level::Low, Speed::Low);
    fpga_creset_b.set_low();

    let ospi_config = embassy_stm32::ospi::Config {
        fifo_threshold: FIFOThresholdLevel::_16Bytes,
        memory_type: MemoryType::Standard,
        device_size: MemorySize::_2MiB,
        chip_select_high_time: ChipSelectHighTime::_1Cycle,
        free_running_clock: false,
        clock_mode: false,
        wrap_size: WrapSize::None,
        clock_prescaler: 3, // 133.33Mhz / (3+1) = 33.3Mhz
        sample_shifting: true,
        delay_hold_quarter_cycle: false,
        chip_select_boundary: 0,
        delay_block_bypass: true,
        max_transfer: 0,
        refresh: 0,
    };

    let ospi = embassy_stm32::ospi::Ospi::new_blocking_quadspi(
        p.OCTOSPI2,
        p.PF4, // P2_CLK
        p.PD4, // P1_IO4
        p.PH3, // P1_IO5
        p.PC3, // P1_IO6
        p.PE10, // P1_IO7
        p.PG12, // P2_NCS
        ospi_config,
    );

    let mut flash = FlashMemory::new(ospi).await;

    loop {
        let flash_id = flash.read_id();
        info!("FLASH ID: {=[u8]:x}", flash_id);

        led.toggle();

        Timer::after_millis(100).await;
    }

    // Flash chip is a 16MBit W25Q16JV-IQ (W25Q16JVUXIQ)
}

const CMD_READ_ID: u8 = 0x9F;
pub struct FlashMemory<I: Instance> {
    ospi: Ospi<'static, I, Blocking>,
}

impl<I: Instance> FlashMemory<I> {
    pub async fn new(ospi: Ospi<'static, I, Blocking>) -> Self {
        let memory = Self { ospi };

        memory
    }

    pub fn read_id(&mut self) -> [u8; 3] {
        let mut buffer = [0; 3];
        let transaction: TransferConfig = TransferConfig {
            iwidth: OspiWidth::SING,
            isize: AddressSize::_8Bit,
            adwidth: OspiWidth::NONE,
            dwidth: OspiWidth::SING,
            instruction: Some(CMD_READ_ID as u32),
            ..Default::default()
        };
        // info!("Reading id: 0x{:X}", transaction.instruction);
        self.ospi.blocking_read(&mut buffer, transaction).unwrap();
        buffer
    }
}


mod rcc_setup {

    use embassy_stm32::rcc::{Hse, HseMode, *};
    use embassy_stm32::time::Hertz;
    use embassy_stm32::{Config, Peripherals};
    use embassy_stm32::rcc::mux::Fmcsel;

    /// Sets up clocks for the stm32h735g mcu
    /// change this if you plan to use a different microcontroller
    pub fn stm32h735g_init() -> Peripherals {

        // setup power and clocks for an stm32h735g-dk run from an external 25 Mhz external oscillator
        let mut config = Config::default();
        config.rcc.hse = Some(Hse {
            freq: Hertz::mhz(50),
            mode: HseMode::Oscillator,
        });
        config.rcc.hsi = None;
        config.rcc.csi = false;
        config.rcc.pll1 = Some(Pll {
            source: PllSource::Hse,
            prediv: PllPreDiv::Div4, // 12.5Mhz
            mul: PllMul::Mul44,     // 550Mhz
            divp: Some(PllDiv::Div1), // 550Mhz
            divq: Some(PllDiv::Div4), // 110Mhz
            divr: Some(PllDiv::Div2), // 275Mhz
        });
        config.rcc.pll2 = Some(Pll {
            source: PllSource::Hse,
            prediv: PllPreDiv::Div5, // 10Mhz
            mul: PllMul::Mul40,      // 400Mhz
            divp: Some(PllDiv::Div5), // 80Mhz
            divq: Some(PllDiv::Div2), // 200Mhz
            divr: Some(PllDiv::Div3), // 133.33Mhz (for OSPI)
        });
        // numbers adapted from Drivers/BSP/STM32H735G-DK/stm32h735g_discovery_lcd.c
        // MX_LTDC_ClockConfig
        config.rcc.pll3 = Some(Pll {
            source: PllSource::Hse,
            prediv: PllPreDiv::Div25, // 2Mhz
            mul: PllMul::Mul96,     // 192Mhz
            divp: Some(PllDiv::Div1), // 192Mhz
            divq: Some(PllDiv::Div8), // 24Mhz
            divr: Some(PllDiv::Div4), // 48Mhz
        });
        config.rcc.voltage_scale = VoltageScale::Scale0;
        config.rcc.supply_config = SupplyConfig::DirectSMPS;
        config.rcc.sys = Sysclk::Pll1P; // 550Mhz
        config.rcc.d1c_pre = AHBPrescaler::Div1; // 550Mhz
        config.rcc.ahb_pre = AHBPrescaler::Div2; // 275Mhz
        config.rcc.apb1_pre = APBPrescaler::Div2; // 137.5Mhz
        config.rcc.apb2_pre = APBPrescaler::Div2; // 137.5Mhz
        config.rcc.apb3_pre = APBPrescaler::Div2; // 137.5Mhz
        config.rcc.apb4_pre = APBPrescaler::Div2; // 137.5Mhz

        config.rcc.mux.octospisel = Fmcsel::Pll2R; // 133.33Mhz

        embassy_stm32::init(config)
    }
}
