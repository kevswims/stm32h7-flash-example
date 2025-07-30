#![no_std]
#![no_main]

use embassy_stm32::{
    gpio::{Level, Output, Speed},
    pac,
};
use flash_algorithm::*;
use rtt_target::{rprintln, rtt_init_print};

use flash_lib::{FlashMemoryResources, SpiFlashMemory, MACRONIX_ID, MEMORY_MAPPED_FLASH_ADDRESS};

struct Algorithm {
    flash: SpiFlashMemory,
    led: Output<'static>,
}

algorithm!(Algorithm, {
    flash_address: MEMORY_MAPPED_FLASH_ADDRESS,
    flash_size: 0x02000000,
    // The actual page size is 0x100 bytes but there's no reason we can't write mote bytes at once
    // which greatly improves performance since there are fewer round trip calls with the probe.
    page_size: 0x1000,
    empty_value: 0xFF,
    sectors: [{
        size: 0x1000,
        address: 0x0,
    }]
});

/// Enables the GPIOs needed for the flash and a status LED.
fn enable_gpios() {
    pac::RCC.ahb4enr().modify(|w| {
        w.set_gpiopen(true);
        w.set_gpiooen(true);
        w.set_gpioden(true);
        w.set_gpionen(true);
    });
    pac::PWR.csr2().modify(|w| {
        w.set_en_xspim1(true);
        w.set_en_xspim2(true);
    })
}

impl FlashAlgorithm for Algorithm {
    fn new(_address: u32, _clock: u32, _function: Function) -> Result<Self, ErrorCode> {
        // rtt_init_print!();
        // rprintln!("Init");

        // return Err(ErrorCode::new(0x02).unwrap());

        enable_gpios();

        // Steal the peripherals since we don't want any of the initialization code to run in this.
        // We just need the peripherals to access the pins for the flash and the LED.
        let p = unsafe { embassy_stm32::Peripherals::steal() };

        let led = Output::new(p.PD10, Level::High, Speed::Low);

        let r = FlashMemoryResources {
            spi: p.XSPI2,
            clk: p.PN6,
            d0: p.PN2,
            d1: p.PN3,
            d2: p.PN4,
            d3: p.PN5,
            d4: p.PN8,
            d5: p.PN9,
            d6: p.PN10,
            d7: p.PN11,
            ncs: p.PN1,
        };

        let mut flash = SpiFlashMemory::new(r);

        let flash_id = flash.read_id();

        if flash_id[0] != MACRONIX_ID {
            return Err(ErrorCode::new(flash_id[0] as u32).unwrap());
        }

        Ok(Self { flash, led })
    }

    fn erase_all(&mut self) -> Result<(), ErrorCode> {
        // rprintln!("Erase All");
        self.flash.erase_chip();
        Ok(())
    }

    fn erase_sector(&mut self, addr: u32) -> Result<(), ErrorCode> {
        self.led.set_low();
        // rprintln!("Erase sector addr:{}", addr);
        self.flash.erase_sector(addr - MEMORY_MAPPED_FLASH_ADDRESS);
        self.led.set_high();

        Ok(())
    }

    fn program_page(&mut self, addr: u32, data: &[u8]) -> Result<(), ErrorCode> {
        // rprintln!("Program Page addr:{} size:{}", addr, data.len());
        self.led.set_low();
        self.flash
            .write_memory(addr - MEMORY_MAPPED_FLASH_ADDRESS, data);
        self.led.set_high();
        Ok(())
    }
}

impl Drop for Algorithm {
    fn drop(&mut self) {}
}
