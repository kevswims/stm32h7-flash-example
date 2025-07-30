#![no_main]
#![no_std]

//! This example tests the flash memory driver by writing and reading back data from the flash memory.
//! It also has some throughput tests to measure the performance of the driver.

use defmt::{error, info};
use embassy_executor::Spawner;
use embassy_time::Instant;

use defmt_rtt as _;
use flash_lib::{MEMORY_MAPPED_FLASH_ADDRESS, SpiFlashMemory};
use panic_probe as _;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let r = flash_lib::init();

    let mut flash = SpiFlashMemory::new(r.flash_memory);

    let flash_id = flash.read_id();
    info!("FLASH ID: {=[u8]:x}", flash_id);

    // let mut flash = flash.into_octo();

    // Erase the first sector.
    flash.erase_sector(0);

    // Write a full sector
    let mut wr_buf = [0u8; 0x1000];
    for i in 0..0x1000 {
        wr_buf[i] = (i & 0xFF) as u8;
    }

    let start = Instant::now();
    flash.write_memory(0, &wr_buf);
    let elapsed = start.elapsed();
    info!("Wrote 4k bytes in {} us", elapsed.as_micros());

    // Read back the first 8 bytes with the fast read command and verify them.
    let mut rd_buf = [0u8; 8];
    flash.read_memory(0, &mut rd_buf);
    info!("WRITE BUF: {=[u8]:#X}", wr_buf[..8]);
    info!("READ BUF: {=[u8]:#X}", rd_buf);

    // Enable memory mapped mode
    flash.enable_mm();
    info!("Enabled memory mapped mode");

    let flash_beginning = MEMORY_MAPPED_FLASH_ADDRESS as *const u32;

    let first_u32 = unsafe { *(flash_beginning) };
    info!("first_u32 {:08x}", first_u32);
    // assert_eq!(first_u32, 0x03020100);

    let second_u32 = unsafe { *(flash_beginning.offset(1)) };
    // assert_eq!(second_u32, 0x07060504);
    info!("second_u32 {:08x}", second_u32);

    // Speed test, read back 1024 u32 (4k bytes) from the memory mapped area.
    let mut rd_buf = [0u32; 1024];

    let start = Instant::now();
    for (pos, val) in rd_buf.iter_mut().enumerate() {
        *val = unsafe { *(flash_beginning.offset(pos as isize)) };
    }
    let elapsed = start.elapsed();
    info!("Read 1024 u32 in {} us", elapsed.as_micros());

    // Verify the read data.
    for (pos, val) in rd_buf.iter().enumerate() {
        let byte0 = ((pos * 4) & 0xFF) as u32;
        let byte1 = (byte0 + 1) << 8;
        let byte2 = (byte0 + 2) << 16;
        let byte3 = (byte0 + 3) << 24;
        let expected = byte0 + byte1 + byte2 + byte3;
        // info!(
        //     "pos: {}, val: {:08x}, expected: {:08x}",
        //     pos, *val, expected
        // );
        if *val != expected {
            error!(
                "Mismatch at pos {}: expected {:08x}, got {:08x}",
                pos, expected, *val
            );
            panic!();
        }
    }

    flash.disable_mm();
    info!("Disabled memory mapped mode");

    info!("DONE");

    let future = core::future::pending();
    let () = future.await;
    defmt::unreachable!();
}
