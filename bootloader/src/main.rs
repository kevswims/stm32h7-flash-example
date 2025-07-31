#![no_main]
#![no_std]

use flash_lib::{self, SpiFlashMemory, OpiFlashMemory, MEMORY_MAPPED_FLASH_ADDRESS};

use embassy_executor::Spawner;
use panic_probe as _;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let r = flash_lib::init();

    let mut flash = SpiFlashMemory::new(r.flash_memory);


    let flash_id = flash.read_id();

    let mut flash = flash.into_octo();

    flash.enable_mm();

    let mut cor = cortex_m::Peripherals::take().unwrap();

    // Enable instruction and data caches on the M7 core. This greatly improves performance.
    // cor.SCB.enable_icache();
    // TODO: Enable data cache once async SPI/I2C works with it (DMA invalidating correctly)
    // cor.SCB.enable_dcache(&mut cor.CPUID);

    unsafe {
        // Set's the vector table offset register to the start of the flash memory.
        cor.SCB.vtor.write(MEMORY_MAPPED_FLASH_ADDRESS);
        // Bootload the flash memory by jumping to the start of the flash memory.
        cortex_m::asm::bootload(MEMORY_MAPPED_FLASH_ADDRESS as *const u32);
    }
}
