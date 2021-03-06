# USB Blaster for Embedded Devices

[![Latest version](https://img.shields.io/crates/v/usbd-blaster.svg)](https://crates.io/crates/usbd-blaster)
[![Documentation](https://docs.rs/usbd-blaster/badge.svg)](https://docs.rs/usbd-blaster)
![License](https://img.shields.io/crates/l/usbd-blaster.svg)

## What is this?

A crate for emulating a USB Blaster device, written in [Rust](https://www.rust-lang.org/).

For the [Arduino MKR Vidor 4000](https://www.arduino.cc/en/Guide/MKRVidor4000), you can use this to program the onboard [FPGA](https://en.wikipedia.org/wiki/Field-programmable_gate_array) with [Quartus](https://en.wikipedia.org/wiki/Intel_Quartus_Prime).

## Usage

### Requirements

* Rust language (rustup, cargo)
* Embedded compiler toolchain
    * for ARM: arm-none-eabi-gcc (ArchLinux users, get gcc-arm-none-eabi-bin) and rustup target add thumbv6m-none-eabi
* Board flashing tool
    * for Atmel SAM: [Atmel SAM flashing tool](https://github.com/shumatech/BOSSA) (aka bossac, comes in Arduino tools)

### Flashing the USB Blaster

#### Arduino MKR Vidor 4000
```bash
RUSTFLAGS='-C link-arg=-Tlink.x' cargo build --release --target thumbv6m-none-eabi --example arduino_mkrvidor4000
arm-none-eabi-objcopy -O binary target/thumbv6m-none-eabi/release/usbblaster-rs target/usbblaster-rs.bin
# Manual step: push reset button twice in quick succession to enter flash mode
bossac -i -d -U true -i -e -w -v target/usbblaster-rs.bin -R
```

### Using the USB Blaster

#### Intel (Altera) Quartus

```bash
# Verify that the blaster exists
jtagconfig
# Flash your FPGA
quartus_pgm -m jtag -o 'p;project-name.sof'
```

#### OpenOCD

```bash
openocd -f altera-usb-blaster.cfg
```

Example configuration:

```
interface usb_blaster
init
scan_chain
svf project.svf
exit
```

Make sure you've enabled SVF file generation, and change project.svf to the name of your project.

You can safely ignore the following error:

`Error: IR capture error at bit 2, saw 0x3FFFFFFFFFFFFD55 not 0x...3`

This seems to happen on other USB blasters too. If you know why this is and can fix it, feel free to open a PR.

## How it works

### USB

The board is set up as a USB device with the same VendorId and ProductId as an Altera USB Blaster.

The blaster communicates via a vendor-specific interface (Class = 255, SubClass = 255, Protocol = 255). When vendor-typed control requests are received, it emulates the ROM and the responses of the [FTDI245 chip](https://www.ftdichip.com/Products/ICs/FT245R.htm).

Just like the FT245, endpoint 1 is input-only and endpoint 2 is output-only. These are used to control blaster operation.

### Blaster

The blaster has two operating modes: bit-bang (default) or shift. In bit-bang, there is direct control of the JTAG lines; every received byte translates to instructions on how to drive TDI/TMS/TCK. It also contains flags for whether this instruction is a read or write, and if the blaster should switch to shift mode and shift out the next n bytes. In shift mode, the blaster will shift out the next n (anywhere from 0 to 63) received bytes to the TDI line.

Bit-bang mode is useful for JTAG control, shift mode is useful for a bulk transfer like writing an FPGA bitstream.

## Quirks/ Things to be aware of

This crate does JTAG only. These other pins are ignored, because [they are not part of JTAG](https://www.intel.com/content/dam/www/programmable/us/en/pdfs/literature/ug/ug_usb_blstr.pdf#_OPENTOPIC_TOC_PROCESSING_d116e1073)

- Active Serial (AS) mode
    - active-low chip enable (nCE)
    - active-low chip select (nCS)
    - active serial data out (DATAOUT)
- Passive Serial (PS) mode
    - active-low configuration status (nSTATUS)

## Special Thanks

* [Martino Facchin](https://github.com/facchinm)
