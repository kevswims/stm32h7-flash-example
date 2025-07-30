#![no_std]

//! For Nucleo STM32H7S3L8 MB1737, has MX25UW25645GXDI00

use assign_resources::assign_resources;
use embassy_stm32::peripherals;
use embassy_stm32::{
    Config, Peri,
    mode::Blocking,
    rcc,
    time::Hertz,
    xspi::{self, AddressSize, DummyCycles, TransferConfig, Xspi, XspiWidth},
};

use core::cmp::min;

/// Gives the underlying type for a `Peri` peripheral reference.
#[macro_export]
macro_rules! peri_type {
    ($t:ty) => {
        <$t as core::ops::Deref>::Target
    };
}

assign_resources! {
    flash_memory: FlashMemoryResources {
        spi: XSPI2 = FlashMemorySpi,
        clk: PN6 = FlashMemoryClk,
        d0: PN2 = FlashMemoryD0,
        d1: PN3 = FlashMemoryD1,
        d2: PN4 = FlashMemoryD2,
        d3: PN5 = FlashMemoryD3,
        d4: PN8 = FlashMemoryD4,
        d5: PN9 = FlashMemoryD5,
        d6: PN10 = FlashMemoryD6,
        d7: PN11 = FlashMemoryD7,
        ncs: PN1 = FlashMemoryNcs,
    }
}

fn configure_rcc(rcc: &mut rcc::Config) {
    use embassy_stm32::rcc::{
        AHBPrescaler, APBPrescaler, Hse, HseMode, Pll, PllDiv, PllMul, PllPreDiv, PllSource,
        Sysclk, VoltageScale,
    };

    rcc.hse = Some(Hse {
        freq: Hertz(24_000_000),
        mode: HseMode::Oscillator,
    });
    rcc.pll1 = Some(Pll {
        source: PllSource::HSE,
        prediv: PllPreDiv::DIV3,
        mul: PllMul::MUL150,
        divp: Some(PllDiv::DIV2),
        divq: None,
        divr: None,
        divs: None,
        divt: None,
    });
    rcc.sys = Sysclk::PLL1_P; // 600 Mhz
    rcc.ahb_pre = AHBPrescaler::DIV2; // 300 Mhz
    rcc.apb1_pre = APBPrescaler::DIV2; // 150 Mhz
    rcc.apb2_pre = APBPrescaler::DIV2; // 150 Mhz
    rcc.apb4_pre = APBPrescaler::DIV2; // 150 Mhz
    rcc.apb5_pre = APBPrescaler::DIV2; // 150 Mhz
    rcc.voltage_scale = VoltageScale::HIGH;
}

pub fn init() -> AssignedResources {
    let mut config = Config::default();
    configure_rcc(&mut config.rcc);

    let p = embassy_stm32::init(config);

    split_resources!(p)
}

/// Max size (in bytes) that can be written in a single page program operation.
const MEMORY_PAGE_SIZE: usize = 256;

/// The address in memory where the flash chip is mapped when in memory mapped mode.
/// This is the address for the XSPI2 peripheral
pub const MEMORY_MAPPED_FLASH_ADDRESS: u32 = 0x70000000;

/// ID for the Macronix MX25UW25645GXDI00 flash chip.
pub const MACRONIX_ID: u8 = 0xC2;

/// Implementation of access to flash chip using SPI.
///
/// Chip commands are hardcoded as it depends on used chip.
/// This targets a MX25UW25645GXDI00.
pub struct SpiFlashMemory {
    xspi: Xspi<'static, peri_type!(FlashMemorySpi), Blocking>,
}

/// Implementation of access to flash chip using Octo SPI.
///
/// Chip commands are hardcoded as it depends on used chip.
/// This targets a MX25UW25645GXDI00.
pub struct OpiFlashMemory {
    xspi: Xspi<'static, peri_type!(FlashMemorySpi), Blocking>,
}

/// SPI mode commands for MX25UW25645G flash memory
#[allow(dead_code)]
#[repr(u8)]
enum SpiCommand {
    // Array access commands
    /// Read data bytes using 3-byte address (up to 50 MHz)
    Read3B = 0x03,
    /// Fast read data bytes using 3-byte address with 8 dummy cycles (up to 133 MHz)
    FastRead3B = 0x0B,
    /// Program 1-256 bytes of data using 3-byte address
    PageProgram3B = 0x02,
    /// Erase 4KB sector using 3-byte address
    SectorErase3B = 0x20,
    /// Erase 64KB block using 3-byte address
    BlockErase3B = 0xD8,
    /// Read data bytes using 4-byte address (up to 50 MHz)
    Read4B = 0x13,
    /// Fast read data bytes using 4-byte address with 8 dummy cycles (up to 133 MHz)
    FastRead4B = 0x0C,
    /// Program 1-256 bytes of data using 4-byte address
    PageProgram4B = 0x12,
    /// Erase 4KB sector using 4-byte address
    SectorErase4B = 0x21,
    /// Erase 64KB block using 4-byte address
    BlockErase4B = 0xDC,
    /// Erase entire chip (only if no blocks are protected)
    ChipErase = 0x60,

    // Write Buffer Access commands
    /// Read data from the 256-byte page buffer
    ReadBuffer = 0x25,
    /// Initialize write-to-buffer sequence, clears buffer and writes initial data
    WriteBufferInitial = 0x22,
    /// Continue writing data to buffer (used between WRBI and WRCF)
    WriteBufferContinue = 0x24,
    /// Confirm write operation, programs buffer contents to flash array
    WriteBufferConfirm = 0x31,

    // Device operation commands
    /// Set Write Enable Latch (WEL) bit, required before write/program/erase operations
    WriteEnable = 0x06,
    /// Clear Write Enable Latch (WEL) bit
    WriteDisable = 0x04,
    /// Select write protection mode (BP mode or Advanced Sector Protection)
    WriteProtectSelection = 0x68,
    /// Suspend ongoing program or erase operation to allow read access
    ProgramEraseSuspend = 0xB0,
    /// Resume suspended program or erase operation
    ProgramEraseResume = 0x30,
    /// Enter deep power-down mode for minimum power consumption
    DeepPowerDown = 0xB9,
    /// Exit deep power-down mode and return to standby
    ReleaseFromDeepPowerDown = 0xAB,
    /// No operation, can terminate Reset Enable command
    NoOperation = 0x00,
    /// Enable reset operation (must precede Reset Memory command)
    ResetEnable = 0x66,
    /// Reset device to power-on state (requires prior Reset Enable)
    ResetMemory = 0x99,
    /// Protect all sectors using Dynamic Protection Bits (DPB)
    GangBlockLock = 0x7E,
    /// Unprotect all sectors by clearing Dynamic Protection Bits (DPB)
    GangBlockUnlock = 0x98,

    // Register Access commands
    /// Read 3-byte device identification (manufacturer ID + device ID)
    ReadIdentification = 0x9F,
    /// Read Serial Flash Discoverable Parameters (SFDP) table
    ReadSFDP = 0x5A,
    /// Read 8-bit Status Register (WIP, WEL, BP bits, etc.)
    ReadStatusRegister = 0x05,
    /// Read 8-bit Configuration Register (ODS, TB, PBE bits)
    ReadConfigurationRegister = 0x15,
    /// Write Status and/or Configuration Register (1-2 bytes)
    WriteStatusConfigurationRegister = 0x01,
    /// Read Configuration Register 2 from specified 4-byte address
    ReadConfigurationRegister2 = 0x71,
    /// Write Configuration Register 2 to specified 4-byte address
    WriteConfigurationRegister2 = 0x72,
    /// Read 8-bit Security Register (protection status, suspend bits)
    ReadSecurityRegister = 0x2B,
    /// Write Security Register to set customer lock-down bit
    WriteSecurityRegister = 0x2F,
    /// Read 32-bit Fast Boot Register (boot address and configuration)
    ReadFastBootRegister = 0x16,
    /// Write 32-bit Fast Boot Register
    WriteFastBootRegister = 0x17,
    /// Erase Fast Boot Register (disable fast boot feature)
    EraseFastBootRegister = 0x18,
    /// Set burst/wrap length for read operations (16/32/64 bytes)
    SetBurstLength = 0xC0,
    /// Enter 8K-bit secured OTP mode for programming unique identifiers
    EnterSecuredOTP = 0xB1,
    /// Exit secured OTP mode and return to main array access
    ExitSecuredOTP = 0xC1,
    /// Write Lock Register to control SPB protection mode
    WriteLockRegister = 0x2C,
    /// Read Lock Register status
    ReadLockRegister = 0x2D,
    /// Program Solid Protection Bit (SPB) for specified sector/block
    WriteSPB = 0xE3,
    /// Erase all Solid Protection Bits (SPB)
    EraseSPB = 0xE4,
    /// Read Solid Protection Bit (SPB) status for specified sector/block
    ReadSPB = 0xE2,
    /// Write Dynamic Protection Bit (DPB) for specified sector
    WriteDPB = 0xE1,
    /// Read Dynamic Protection Bit (DPB) status for specified sector
    ReadDPB = 0xE0,
    /// Read 64-bit password register (only in Solid Protection mode)
    ReadPassword = 0x27,
    /// Write 64-bit password register
    WritePassword = 0x28,
    /// Unlock SPB operations using 64-bit password
    PasswordUnlock = 0x29,
}

/// OPI mode commands for MX25UW25645G flash memory
#[allow(dead_code)]
#[repr(u16)]
enum OpiCommand {
    // Array access commands
    /// Read data using 8 I/O lines in STR mode with configurable dummy cycles (up to 200 MHz)
    OctaRead = 0xEC13,
    /// Read data using 8 I/O lines in DTR mode with configurable dummy cycles (up to 200 MHz)
    OctaDTRRead = 0xEE11,
    /// Program 1-256 bytes using 4-byte address and 8 I/O lines
    PageProgram4B = 0x12ED,
    /// Erase 4KB sector using 4-byte address
    SectorErase4B = 0x21DE,
    /// Erase 64KB block using 4-byte address
    BlockErase4B = 0xDC23,
    /// Erase entire chip (only if no blocks are protected)
    ChipErase = 0x609F,

    // Write Buffer Access commands
    /// Read data from the 256-byte page buffer using 4-byte address
    ReadBuffer = 0x25DA,
    /// Initialize interruptible write-to-buffer sequence with 4-byte address
    WriteBufferInitial = 0x22DD,
    /// Continue writing data to buffer during interruptible sequence
    WriteBufferContinue = 0x24DB,
    /// Confirm and execute write operation from buffer to flash array
    WriteBufferConfirm = 0x31CE,

    // Device operation commands
    /// Set Write Enable Latch (WEL) bit, required before write/program/erase operations
    WriteEnable = 0x06F9,
    /// Clear Write Enable Latch (WEL) bit, aborts write-to-buffer sequence
    WriteDisable = 0x04FB,
    /// Select write protection mode (BP mode or Advanced Sector Protection) - OTP bit
    WriteProtectSelection = 0x6897,
    /// Suspend ongoing program or erase operation to allow read from other banks
    ProgramEraseSuspend = 0xB04F,
    /// Resume suspended program or erase operation
    ProgramEraseResume = 0x30CF,
    /// Enter deep power-down mode for minimum power consumption
    DeepPowerDown = 0xB946,
    /// Exit deep power-down mode and return to standby
    ReleaseFromDeepPowerDown = 0xAB54,
    /// No operation, can terminate Reset Enable command
    NoOperation = 0x00FF,
    /// Enable reset operation (must precede Reset Memory command)
    ResetEnable = 0x6699,
    /// Reset device to power-on state, clears volatile settings
    ResetMemory = 0x9966,
    /// Protect all sectors using Dynamic Protection Bits (DPB)
    GangBlockLock = 0x7E81,
    /// Unprotect all sectors by clearing Dynamic Protection Bits (DPB)
    GangBlockUnlock = 0x9867,

    // Register Access commands
    /// Read 3-byte device identification with 4-byte dummy address
    ReadIdentification = 0x9F60,
    /// Read Serial Flash Discoverable Parameters (SFDP) table with 4-byte address
    ReadSFDP = 0x5AA5,
    /// Read 8-bit Status Register with 4-byte dummy address
    ReadStatusRegister = 0x05FA,
    /// Read 8-bit Configuration Register with specific address (00000001h)
    ReadConfigurationRegister = 0x15EA,
    /// Write 8-bit Status Register with specific address (00000000h) or Configuration Register with address (00000001h)
    WriteStatusConfigurationRegister = 0x01FE,
    /// Read Configuration Register 2 from specified 4-byte address
    ReadConfigurationRegister2 = 0x718E,
    /// Write Configuration Register 2 to specified 4-byte address
    WriteConfigurationRegister2 = 0x728D,
    /// Read 8-bit Security Register with 4-byte dummy address
    ReadSecurityRegister = 0x2BD4,
    /// Write Security Register to set customer lock-down bit
    WriteSecurityRegister = 0x2FD0,
    /// Set burst/wrap length for read operations with 4-byte dummy address
    SetBurstLength = 0xC03F,
    /// Read 32-bit Fast Boot Register with 4-byte dummy address
    ReadFastBootRegister = 0x16E9,
    /// Write 32-bit Fast Boot Register with 4-byte dummy address
    WriteFastBootRegister = 0x17E8,
    /// Erase Fast Boot Register (disable fast boot feature)
    EraseFastBootRegister = 0x18E7,
    /// Enter 8K-bit secured OTP mode for programming unique identifiers
    EnterSecuredOTP = 0xB14E,
    /// Exit secured OTP mode and return to main array access
    ExitSecuredOTP = 0xC13E,
    /// Write Lock Register to control SPB protection mode with 4-byte dummy address
    WriteLockRegister = 0x2CD3,
    /// Read Lock Register status with 4-byte dummy address
    ReadLockRegister = 0x2DD2,
    /// Program Solid Protection Bit (SPB) for specified 4-byte address
    WriteSPB = 0xE31C,
    /// Erase all Solid Protection Bits (SPB)
    EraseSPB = 0xE41B,
    /// Read Solid Protection Bit (SPB) status for specified 4-byte address
    ReadSPB = 0xE21D,
    /// Write Dynamic Protection Bit (DPB) for specified 4-byte address
    WriteDPB = 0xE11E,
    /// Read Dynamic Protection Bit (DPB) status for specified 4-byte address
    ReadDPB = 0xE01F,
    /// Read 64-bit password register with 4-byte dummy address and 20 dummy cycles
    ReadPassword = 0x27D8,
    /// Write 64-bit password register with 4-byte dummy address
    WritePassword = 0x28D7,
    /// Unlock SPB operations using 64-bit password with 4-byte dummy address
    PasswordUnlock = 0x29D6,
}

impl SpiFlashMemory {
    pub fn new(r: FlashMemoryResources) -> Self {
        use xspi::{ChipSelectHighTime, FIFOThresholdLevel, MemorySize, MemoryType, WrapSize};

        let config = xspi::Config {
            fifo_threshold: FIFOThresholdLevel::_4Bytes,
            memory_type: MemoryType::Macronix,
            delay_hold_quarter_cycle: true,
            device_size: MemorySize::_32MiB,
            chip_select_high_time: ChipSelectHighTime::_2Cycle,
            free_running_clock: false,
            clock_mode: false,
            wrap_size: WrapSize::None,
            // 300 MHz clock / (3 + 1) = 75 MHz. This is above the max for READ instructions so the
            // FAST READ must be used. The nucleo board's flash  can run at up to 133 MHz in SPI mode
            // and 200 MHz in OPI mode. This clock prescaler must be even otherwise the clock will not
            // have symmetric high and low times.
            // The clock can also be fed by one of the PLLs to allow for more flexible clock rates.
            clock_prescaler: 3,
            sample_shifting: false,
            chip_select_boundary: 0,
            max_transfer: 0,
            refresh: 0,
        };

        let xspi = Xspi::new_blocking_xspi(
            r.spi, r.clk, r.d0, r.d1, r.d2, r.d3, r.d4, r.d5, r.d6, r.d7, r.ncs, config,
        );

        let mut memory = Self { xspi };

        memory.reset_memory();
        memory
    }

    pub fn disable_mm(&mut self) {
        self.xspi.disable_memory_mapped_mode();
    }

    pub fn enable_mm(&mut self) {
        let read_config = TransferConfig {
            iwidth: XspiWidth::SING,
            isize: AddressSize::_8bit,
            adwidth: XspiWidth::SING,
            adsize: AddressSize::_32bit,
            dwidth: XspiWidth::SING,
            instruction: Some(SpiCommand::FastRead4B as u32),
            dummy: DummyCycles::_8,
            ..Default::default()
        };

        let write_config = TransferConfig {
            iwidth: XspiWidth::SING,
            isize: AddressSize::_8bit,
            adwidth: XspiWidth::SING,
            adsize: AddressSize::_32bit,
            dwidth: XspiWidth::SING,
            instruction: Some(SpiCommand::PageProgram4B as u32),
            dummy: DummyCycles::_0,
            ..Default::default()
        };
        self.xspi
            .enable_memory_mapped_mode(read_config, write_config)
            .unwrap();
    }

    pub fn into_octo(mut self) -> OpiFlashMemory {
        self.enable_opi_mode();
        OpiFlashMemory { xspi: self.xspi }
    }

    fn enable_opi_mode(&mut self) {
        let cr2_0 = self.read_cr2(0);
        self.enable_write();
        self.write_cr2(0, cr2_0 | 0x01); // Set bit 0 to enable octo SPI in STR
    }

    fn exec_command(&mut self, cmd: u8) {
        let transaction = TransferConfig {
            iwidth: XspiWidth::SING,
            adwidth: XspiWidth::NONE,
            // adsize: AddressSize::_24bit,
            dwidth: XspiWidth::NONE,
            instruction: Some(cmd as u32),
            address: None,
            dummy: DummyCycles::_0,
            ..Default::default()
        };
        // info!("Excuting command: {:x}", transaction.instruction);
        self.xspi.blocking_command(&transaction).unwrap();
    }

    pub fn reset_memory(&mut self) {
        self.exec_command(SpiCommand::ResetEnable as u8);
        self.exec_command(SpiCommand::ResetMemory as u8);
        self.wait_write_finish();
    }

    pub fn enable_write(&mut self) {
        self.exec_command(SpiCommand::WriteEnable as u8);
    }

    pub fn read_id(&mut self) -> [u8; 3] {
        let mut buffer = [0; 3];
        let transaction: TransferConfig = TransferConfig {
            iwidth: XspiWidth::SING,
            isize: AddressSize::_8bit,
            adwidth: XspiWidth::NONE,
            dwidth: XspiWidth::SING,
            instruction: Some(SpiCommand::ReadIdentification as u32),
            ..Default::default()
        };
        self.xspi.blocking_read(&mut buffer, transaction).unwrap();
        buffer
    }

    pub fn read_memory(&mut self, addr: u32, buffer: &mut [u8]) {
        let transaction = TransferConfig {
            iwidth: XspiWidth::SING,
            adwidth: XspiWidth::SING,
            adsize: AddressSize::_32bit,
            dwidth: XspiWidth::SING,
            instruction: Some(SpiCommand::FastRead4B as u32),
            dummy: DummyCycles::_8,
            address: Some(addr),
            ..Default::default()
        };

        self.xspi.blocking_read(buffer, transaction).unwrap();
    }

    fn wait_write_finish(&mut self) {
        while (self.read_sr() & 0x01) != 0 {}
    }

    fn perform_erase(&mut self, addr: u32, cmd: u8) {
        let transaction = TransferConfig {
            iwidth: XspiWidth::SING,
            adwidth: XspiWidth::SING,
            adsize: AddressSize::_32bit,
            dwidth: XspiWidth::NONE,
            instruction: Some(cmd as u32),
            address: Some(addr),
            dummy: DummyCycles::_0,
            ..Default::default()
        };
        self.enable_write();
        self.xspi.blocking_command(&transaction).unwrap();
        self.wait_write_finish();
    }

    pub fn erase_sector(&mut self, addr: u32) {
        self.perform_erase(addr, SpiCommand::SectorErase4B as u8);
    }

    pub fn erase_block_64k(&mut self, addr: u32) {
        self.perform_erase(addr, SpiCommand::BlockErase4B as u8);
    }

    pub fn erase_chip(&mut self) {
        self.enable_write();
        self.exec_command(SpiCommand::ChipErase as u8);
        self.wait_write_finish();
    }

    fn write_page(&mut self, addr: u32, buffer: &[u8], len: usize) {
        assert!(
            (len as u32 + (addr & 0x000000ff)) <= MEMORY_PAGE_SIZE as u32,
            "write_page(): page write length exceeds page boundary (len = {}, addr = {:X}",
            len,
            addr
        );

        let transaction = TransferConfig {
            iwidth: XspiWidth::SING,
            adsize: AddressSize::_32bit,
            adwidth: XspiWidth::SING,
            dwidth: XspiWidth::SING,
            instruction: Some(SpiCommand::PageProgram4B as u32),
            address: Some(addr),
            dummy: DummyCycles::_0,
            ..Default::default()
        };
        self.enable_write();
        self.xspi.blocking_write(buffer, transaction).unwrap();
        self.wait_write_finish();
    }

    pub fn write_memory(&mut self, addr: u32, buffer: &[u8]) {
        let mut left = buffer.len();
        let mut place = addr;
        let mut chunk_start = 0;

        while left > 0 {
            let max_chunk_size = MEMORY_PAGE_SIZE - (place & 0x000000ff) as usize;
            let chunk_size = min(max_chunk_size, left);
            let chunk = &buffer[chunk_start..(chunk_start + chunk_size)];
            self.write_page(place, chunk, chunk_size);
            place += chunk_size as u32;
            left -= chunk_size;
            chunk_start += chunk_size;
        }
    }

    // Note: read_register cannot be used to read the configuration register 2 since there is an
    // address required for that read.
    fn read_register(&mut self, cmd: u8) -> u8 {
        let mut buffer = [0; 1];
        let transaction: TransferConfig = TransferConfig {
            iwidth: XspiWidth::SING,
            isize: AddressSize::_8bit,
            adwidth: XspiWidth::NONE,
            dwidth: XspiWidth::SING,
            instruction: Some(cmd as u32),
            address: None,
            dummy: DummyCycles::_0,
            ..Default::default()
        };
        self.xspi.blocking_read(&mut buffer, transaction).unwrap();
        buffer[0]
    }

    pub fn read_sr(&mut self) -> u8 {
        self.read_register(SpiCommand::ReadStatusRegister as u8)
    }

    pub fn read_cr(&mut self) -> u8 {
        self.read_register(SpiCommand::ReadConfigurationRegister as u8)
    }

    pub fn write_sr_cr(&mut self, sr: u8, cr: u8) {
        let buffer = [sr, cr];
        let transaction: TransferConfig = TransferConfig {
            iwidth: XspiWidth::SING,
            isize: AddressSize::_8bit,
            instruction: Some(SpiCommand::WriteStatusConfigurationRegister as u32),
            adwidth: XspiWidth::NONE,
            dwidth: XspiWidth::SING,
            address: None,
            dummy: DummyCycles::_0,
            ..Default::default()
        };
        self.enable_write();
        self.xspi.blocking_write(&buffer, transaction).unwrap();
        self.wait_write_finish();
    }

    pub fn read_cr2(&mut self, address: u32) -> u8 {
        let mut buffer = [0; 1];
        let transaction: TransferConfig = TransferConfig {
            iwidth: XspiWidth::SING,
            isize: AddressSize::_8bit,
            instruction: Some(SpiCommand::ReadConfigurationRegister2 as u32),
            adsize: AddressSize::_32bit,
            adwidth: XspiWidth::SING,
            dwidth: XspiWidth::SING,
            address: Some(address),
            dummy: DummyCycles::_0,
            ..Default::default()
        };
        self.xspi.blocking_read(&mut buffer, transaction).unwrap();
        buffer[0]
    }

    pub fn write_cr2(&mut self, address: u32, value: u8) {
        let buffer = [value; 1];
        let transaction: TransferConfig = TransferConfig {
            iwidth: XspiWidth::SING,
            isize: AddressSize::_8bit,
            instruction: Some(SpiCommand::WriteConfigurationRegister2 as u32),
            adsize: AddressSize::_32bit,
            adwidth: XspiWidth::SING,
            dwidth: XspiWidth::SING,
            address: Some(address),
            dummy: DummyCycles::_0,
            ..Default::default()
        };
        self.xspi.blocking_write(&buffer, transaction).unwrap();
        self.wait_write_finish();
    }
}

impl OpiFlashMemory {
    pub fn into_spi(mut self) -> SpiFlashMemory {
        self.disable_opi_mode();
        SpiFlashMemory { xspi: self.xspi }
    }

    /// Disable OPI mode and return to SPI
    pub fn disable_opi_mode(&mut self) {
        // Clear SOPI and DOPI bits in CR2 volatile register
        let cr2_0 = self.read_cr2(0x00000000);
        self.write_cr2(0x00000000, cr2_0 & 0xFC); // Clear bits 0 and 1
    }

    /// Enable memory-mapped mode for OPI
    pub fn enable_mm(&mut self) {
        let read_config = TransferConfig {
            iwidth: XspiWidth::OCTO,
            isize: AddressSize::_16bit, // 2-byte command for OPI
            adwidth: XspiWidth::OCTO,
            adsize: AddressSize::_32bit,
            dwidth: XspiWidth::OCTO,
            instruction: Some(OpiCommand::OctaRead as u32),
            dummy: DummyCycles::_20, // Default dummy cycles for OPI
            ..Default::default()
        };

        let write_config = TransferConfig {
            iwidth: XspiWidth::OCTO,
            isize: AddressSize::_16bit,
            adwidth: XspiWidth::OCTO,
            adsize: AddressSize::_32bit,
            dwidth: XspiWidth::OCTO,
            instruction: Some(OpiCommand::PageProgram4B as u32),
            dummy: DummyCycles::_0,
            ..Default::default()
        };

        self.xspi
            .enable_memory_mapped_mode(read_config, write_config)
            .unwrap();
    }

    pub fn disable_mm(&mut self) {
        self.xspi.disable_memory_mapped_mode();
    }

    /// Execute OPI command (2-byte command)
    fn exec_command(&mut self, cmd: OpiCommand) {
        let transaction = TransferConfig {
            iwidth: XspiWidth::OCTO,
            isize: AddressSize::_16bit, // 2-byte command
            adwidth: XspiWidth::NONE,
            dwidth: XspiWidth::NONE,
            instruction: Some(cmd as u32),
            address: None,
            dummy: DummyCycles::_0,
            ..Default::default()
        };
        self.xspi.blocking_command(&transaction).unwrap();
    }

    /// Reset memory using OPI commands
    pub fn reset_memory(&mut self) {
        self.exec_command(OpiCommand::ResetEnable);
        self.exec_command(OpiCommand::ResetMemory);
        self.wait_write_finish();
    }

    /// Enable write using OPI command
    pub fn enable_write(&mut self) {
        self.exec_command(OpiCommand::WriteEnable);
    }

    /// Read device ID in OPI mode
    pub fn read_id(&mut self) -> [u8; 3] {
        let mut buffer = [0; 3];
        let transaction = TransferConfig {
            iwidth: XspiWidth::OCTO,
            isize: AddressSize::_16bit,
            adwidth: XspiWidth::OCTO,
            adsize: AddressSize::_32bit,
            dwidth: XspiWidth::OCTO,
            instruction: Some(OpiCommand::ReadIdentification as u32),
            address: Some(0x00000000), // Dummy address required
            dummy: DummyCycles::_4,
            ..Default::default()
        };
        self.xspi.blocking_read(&mut buffer, transaction).unwrap();
        buffer
    }

    /// Read memory using OPI mode
    pub fn read_memory(&mut self, addr: u32, buffer: &mut [u8]) {
        let transaction = TransferConfig {
            iwidth: XspiWidth::OCTO,
            isize: AddressSize::_16bit,
            adwidth: XspiWidth::OCTO,
            adsize: AddressSize::_32bit,
            dwidth: XspiWidth::OCTO,
            instruction: Some(OpiCommand::OctaRead as u32),
            address: Some(addr),
            dummy: DummyCycles::_20, // Default for 200MHz operation
            ..Default::default()
        };
        self.xspi.blocking_read(buffer, transaction).unwrap();
    }

    /// Wait for write completion using OPI status read
    fn wait_write_finish(&mut self) {
        while (self.read_sr() & 0x01) != 0 {}
    }

    /// Perform erase operation using OPI command
    fn perform_erase(&mut self, addr: u32, cmd: OpiCommand) {
        let transaction = TransferConfig {
            iwidth: XspiWidth::OCTO,
            isize: AddressSize::_16bit,
            adwidth: XspiWidth::OCTO,
            adsize: AddressSize::_32bit,
            dwidth: XspiWidth::NONE,
            instruction: Some(cmd as u32),
            address: Some(addr),
            dummy: DummyCycles::_0,
            ..Default::default()
        };
        self.enable_write();
        self.xspi.blocking_command(&transaction).unwrap();
        self.wait_write_finish();
    }

    /// Erase 4KB sector using OPI
    pub fn erase_sector(&mut self, addr: u32) {
        self.perform_erase(addr, OpiCommand::SectorErase4B);
    }

    /// Erase 64KB block using OPI
    pub fn erase_block_64k(&mut self, addr: u32) {
        self.perform_erase(addr, OpiCommand::BlockErase4B);
    }

    /// Erase entire chip using OPI
    pub fn erase_chip(&mut self) {
        self.enable_write();
        self.exec_command(OpiCommand::ChipErase);
        self.wait_write_finish();
    }

    /// Write single page using OPI
    fn write_page(&mut self, addr: u32, buffer: &[u8], len: usize) {
        assert!(
            (len as u32 + (addr & 0x000000ff)) <= MEMORY_PAGE_SIZE as u32,
            "write_page(): page write length exceeds page boundary (len = {}, addr = {:X})",
            len,
            addr
        );

        let transaction = TransferConfig {
            iwidth: XspiWidth::OCTO,
            isize: AddressSize::_16bit,
            adwidth: XspiWidth::OCTO,
            adsize: AddressSize::_32bit,
            dwidth: XspiWidth::OCTO,
            instruction: Some(OpiCommand::PageProgram4B as u32),
            address: Some(addr),
            dummy: DummyCycles::_0,
            ..Default::default()
        };
        self.enable_write();
        self.xspi.blocking_write(buffer, transaction).unwrap();
        self.wait_write_finish();
    }

    /// Write memory using OPI (handles page boundaries)
    pub fn write_memory(&mut self, addr: u32, buffer: &[u8]) {
        let mut left = buffer.len();
        let mut place = addr;
        let mut chunk_start = 0;

        while left > 0 {
            let max_chunk_size = MEMORY_PAGE_SIZE - (place & 0x000000ff) as usize;
            let chunk_size = min(max_chunk_size, left);
            let chunk = &buffer[chunk_start..(chunk_start + chunk_size)];
            self.write_page(place, chunk, chunk_size);
            place += chunk_size as u32;
            left -= chunk_size;
            chunk_start += chunk_size;
        }
    }

    /// Read register using OPI mode
    fn read_register(&mut self, cmd: OpiCommand, dummy_addr: u32, dummy_cycles: DummyCycles) -> u8 {
        let mut buffer = [0; 1];
        let transaction = TransferConfig {
            iwidth: XspiWidth::OCTO,
            isize: AddressSize::_16bit,
            adwidth: XspiWidth::OCTO,
            adsize: AddressSize::_32bit,
            dwidth: XspiWidth::OCTO,
            instruction: Some(cmd as u32),
            address: Some(dummy_addr),
            dummy: dummy_cycles,
            ..Default::default()
        };
        self.xspi.blocking_read(&mut buffer, transaction).unwrap();
        buffer[0]
    }

    /// Read Status Register using OPI
    pub fn read_sr(&mut self) -> u8 {
        self.read_register(
            OpiCommand::ReadStatusRegister,
            0x00000000, // Dummy address
            DummyCycles::_4,
        )
    }

    /// Read Configuration Register using OPI
    pub fn read_cr(&mut self) -> u8 {
        self.read_register(
            OpiCommand::ReadConfigurationRegister,
            0x00000001, // Address for CR
            DummyCycles::_4,
        )
    }

    /// Write Status/Configuration Register using OPI
    pub fn write_sr_cr(&mut self, sr: u8, cr: u8) {
        let transaction = TransferConfig {
            iwidth: XspiWidth::OCTO,
            isize: AddressSize::_16bit,
            adwidth: XspiWidth::OCTO,
            adsize: AddressSize::_32bit,
            dwidth: XspiWidth::OCTO,
            instruction: Some(OpiCommand::WriteStatusConfigurationRegister as u32),
            address: Some(0x00000000),
            dummy: DummyCycles::_0,
            ..Default::default()
        };

        self.enable_write();
        self.xspi.blocking_write(&[sr, cr], transaction).unwrap();
        self.wait_write_finish();
    }

    /// Read Configuration Register 2 using OPI
    pub fn read_cr2(&mut self, address: u32) -> u8 {
        let mut buffer = [0; 1];
        let transaction = TransferConfig {
            iwidth: XspiWidth::OCTO,
            isize: AddressSize::_16bit,
            adwidth: XspiWidth::OCTO,
            adsize: AddressSize::_32bit,
            dwidth: XspiWidth::OCTO,
            instruction: Some(OpiCommand::ReadConfigurationRegister2 as u32),
            address: Some(address),
            dummy: DummyCycles::_4,
            ..Default::default()
        };
        self.xspi.blocking_read(&mut buffer, transaction).unwrap();
        buffer[0]
    }

    /// Write Configuration Register 2 using OPI
    pub fn write_cr2(&mut self, address: u32, value: u8) {
        let transaction = TransferConfig {
            iwidth: XspiWidth::OCTO,
            isize: AddressSize::_16bit,
            adwidth: XspiWidth::OCTO,
            adsize: AddressSize::_32bit,
            dwidth: XspiWidth::OCTO,
            instruction: Some(OpiCommand::WriteConfigurationRegister2 as u32),
            address: Some(address),
            dummy: DummyCycles::_0,
            ..Default::default()
        };

        self.enable_write();
        self.xspi.blocking_write(&[value], transaction).unwrap();
        self.wait_write_finish();
    }
}
