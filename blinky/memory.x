MEMORY
{
    /* The 0x8000000 address is the internal flash memory that holds the bootloader. This is commented out
       so that the bootloader is not overwritten when flashing the firmware. 
       If you need to do quick tests of small programs running from the main flash, you can uncomment this line
       and comment out the FLASH line below. */
    /* FLASH : ORIGIN = 0x08000000, LENGTH =   64K /* BANK_1 */
    FLASH : ORIGIN = 0x70000000, LENGTH = 16M /* XSPI2 */
    RAM   : ORIGIN = 0x24000000, LENGTH =  456K /* SRAM1 + SRAM2 + SRAM3 + SRAM4 */
}
