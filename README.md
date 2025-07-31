# STM32H7RS External Flash Example

## Overview

This repository provides an example for loading and executing code from an external QSPI/OSPI flash
attached to an STM32H7R/S MCU. The example targets the [NUCLEO-H7S3L8](https://www.st.com/en/evaluation-tools/nucleo-h7s3l8.html) 
development board.

## Process for running from external flash

Many MCUs, especially larger ones, have the ability to execute code that is stored in an external flash
chip. As process nodes shrink, this is becoming even more common since flash is more difficult to manufacture
on smaller nodes. While the process of executing code from the external flash is typically straightforward,
the debugger also needs to be able to load the code into that flash. 

The STM32H7RS series has a very flexible external flash controller which supports a variety of flash interfaces
from a simple SPI flash, all the way up to hexaflash interfaces with 16 data lines to support high bandwidth
applications such as loading graphics for high resolution displays. This flexibilty adds a certain amount of
complexity to the process since each board design will require some modifications to the flash-algorithm and
bootloader to support the flash interface technology used.

To load code into the flash, the debugger will load a small program called a flash algorithm into memory which is 
capable of erasing the flash and copying the binary from memory into the flash. The details of what this small program
does are described in the [CMSIS pack](https://open-cmsis-pack.github.io/Open-CMSIS-Pack-Spec/main/html/flashAlgorithm.html)
documentation.

The STM32H7RS series has a small internal flash (64KB) which is where the MCU will beging executing code from after a
reset. This internal flash needs to be loaded with a bootloader which configures the external flash controller before
jumping to the start of the external flash.

## Project Structure

There are multiple projects in this repo that work together to enable the full system:

### Flash Library

The flash library contains all of the flash-specific code to read, erase, and write to the external flash. This
is the code that would need to be modified to support other flash ICs. There is also a `flash-test` program which
can be used to make sure the library is working correctly and to get some rough performance numbers for the flash.

### Flash Algorithm

The flash algorithm uses the probe-rs flash algorithm library and the flash library to create the flash algorithm
for the debugger. The `template.yaml` file is used to define the memory regions of the MCU and it gets combined with
the algorithm code to create the `definition.yaml` file. This is the file used by the debugger to tell it how to
interact with the MCU.

### Bootloader

The bootloader is a very simple program that gets loaded into the internal boot flash of the MCU. This program
sets up the flash controller and clocks and then jumps to the start of the flash.

### Blinky

The blinky program is a test program that actually gets loaded into the flash to make sure everything is working correctly.

## Running the demo

To run the demo on the Nucleo board, follow these steps:

#### Install the bootloader
```
cd bootloader
cargo run --release
```  
Once the bootloader is loaded it will indicate that the core is locked up. This is because it tried to jump to the 
flash which has nothing loaded on it. This is OK. Just Ctrl-C to exit the debug session.

#### Run Blinky
```
cd blinky
cargo run
```
If everything is working, the blinky code should start running and you will see the defmt logs as LED 1 blinks on the board.

## Adapting the code for other hardware

To run this on other hardware, changes will likely need to be made to work with the flash setup.

#### Update the flash library

The flash library will need to be updated to match the configuration of the flash on the custom board.
After updates have been made, the `flash-test` executable will let you test out that the reads and writes
to the flash are working as expected.

#### Build a new flash-algorithm

Running `cargo run` in the flash-algo directory will generate the flash algorithm code and create a new
`definition.yaml` file in the `target` directory. This new `definition.yaml` file needs to be copied to the
`blinky` directory to replace the one that is there.

:warning: The [target-gen](https://github.com/probe-rs/probe-rs/tree/master/target-gen) command that 
is provided by probe-rs does not seem to report failures correctly so even if it succeeds, there is still 
a possibility that the flash algorithm may not work correctly.

#### Running it

Follow the steps above to install the bootloader and run blinky.

## External Projects

This example leverages the following external projects and tools. For more details on them check out
their respective documentation and repositories.

- [Probe-rs](https://probe.rs/) provides a connection to the ST-Link debugger
- [Embassy](https://embassy.dev/) provides the async runtime and HAL/PAC layers for interfacing with the MCU
- [Probe-rs Flash Algorithm](https://github.com/probe-rs/flash-algorithm) provides a library for writing custom flash algorithms
- [Defmt](https://defmt.ferrous-systems.com/) provides debugging/logging output over the ST-Link
