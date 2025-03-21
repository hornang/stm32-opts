# stm32-opts

This tool allows you to read or write the [`nDBANK`](https://github.com/embassy-rs/stm32-data-generated/blob/aee345990e1a14efd018877b52aff1d63e16bb7f/stm32-metapac/src/registers/flash_f7.rs#L314-L321) bit on STM32F76x/F77x microcontrollers using a debug probe. It is based on the [`probe-rs`](https://github.com/probe-rs/probe-rs) library.

## Usage

To read the current state:

```
stm32-opts --chip STM32F767ZI ndbank
```

To set new value:

```
stm32-opts --chip STM32F767ZI ndbank false
```

## Motivation

While exploring the [STM32F767ZI](https://www.st.com/en/microcontrollers-microprocessors/stm32f767zi.html), I discovered that its flash memory can operate in one of two modes: a 2 MB single-bank mode or a dual-bank mode with 1 MB per bank. The factory setting for this particular chip is the single-bank mode.

In dual-bank mode, the firmware can execute from one bank while data is written to the other bank without stalling the CPU (read-while-write or RWW for short).

This capability is particularly important for real-time applications that need to perform the following tasks while the firmware is executing:

- Download firmware upgrades (swapped by the bootloader at the next boot)
- Update configuration data
- Log data to flash

Changing the flash mode alters the flash memory addressing scheme, which invalidates any firmware written for the previous mode. Therefore, the most practical approach is to configure the desired mode *before* flashing software to the device.

Alternative solutions exist, primarily ST's own [STM32CubeProgrammer](https://www.st.com/en/development-tools/stm32cubeprog.html). However, incorporating a closed-source tool into the development process can be cumbersome. For example, STM32CubeProgrammer requires more than 1 GB of disk space.

## What's next?

Using the Rust embedded ecosystem for your newly configured dual-bank chip has additional challenges:

- `embassy-stm32` [only supports the default flash mode](https://github.com/embassy-rs/stm32-data/issues/531#issuecomment-2429771874).
- `probe-rs` must use [this flash algorithm](https://github.com/probe-rs/probe-rs/blob/91fffa8f04fd0a14617d97f228c6532b2beb26e1/probe-rs/targets/STM32F7_Series.yaml#L4441-L4470) which is not the default algorithm.