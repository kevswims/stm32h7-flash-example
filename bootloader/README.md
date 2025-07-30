# STM32 Bootloader

This project contains the bootloader for the Nucleo STM32H7S board. This bootloader runs from the internal
flash on the MCU and is responsible for initializing the OSPI flash and starting execution from it.

## Building

```bash
cargo build
```

## Running

```bash
cargo run --release
```
