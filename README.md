# BoldOS
[![GitHub issues](https://img.shields.io/github/issues/Wazzaps/bold)](https://github.com/Wazzaps/bold/issues)
[![GitHub forks](https://img.shields.io/github/forks/Wazzaps/bold)](https://github.com/Wazzaps/bold/network)
[![GitHub stars](https://img.shields.io/github/stars/Wazzaps/bold)](https://github.com/Wazzaps/bold/stargazers)
[![GitHub license](https://badgen.net/github/license/Wazzaps/bold)](https://github.com/Wazzaps/bold/blob/master/LICENSE)
[![GitHub Contributors](https://img.shields.io/github/contributors/Wazzaps/bold.svg?style=flat)](https://github.com/Wazzaps/bold/graphs/contributors)

[![Nightly Rust CI Build](https://github.com/Wazzaps/bold/actions/workflows/rust-nightly-ci.yml/badge.svg)](https://github.com/Wazzaps/bold/actions/workflows/rust-nightly-ci.yml)
[![Build BoldOS](https://github.com/Wazzaps/bold/actions/workflows/bold-build.yml/badge.svg)](https://github.com/Wazzaps/bold/actions/workflows/build-bold.yml)

Tested on Raspberry pi 3 on QEMU

Mix of:

- `https://github.com/cs140e/rpi3-rust-template`
- `https://github.com/bztsrc/raspi3-tutorial`
- `https://wiki.osdev.org/Raspberry_Pi_Bare_Bones`
- My ideas
---

## Screenshot

![Screenshot](https://i.ibb.co/93HxpHW/Screenshot-from-2021-08-12-22-43-25.png)

## Development environment (linux) - with GUI

- Install dependencies:
  - `apt install clang llvm binutils-aarch64-linux-gnu dosfstools mtools curl gdb-multiarch qemu-system-aarch64`
- Install rust:
  - `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
    - (Default everything)
  - `rustup component add rust-src`
  - `rustup override set nightly`
- Run it:
  - `cargo run --release`

## Development environment (linux) - without GUI

- Install dependencies:
  - `apt install clang llvm binutils-aarch64-linux-gnu dosfstools mtools curl gdb-multiarch`
  - `apt install --no-install-recommends qemu-system-aarch64`
- Install rust:
  - `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
    - (Default everything)
    - `source $HOME/.cargo/env`
  - `rustup override set nightly`
  - `rustup component add rust-src`
- Run it:
  - `cargo run --release -- -nographic -monitor none`

## Extra stuff

### GDB

- Run the kernel (either `cargo run --release` or `cargo run-stopped`)
- `./scripts/gdb_attach.sh`

### Parsing exceptions

- Copy the "ESR" value
- Run `parse_esr.py`, and paste it in

## Todo

- [x] Physical page allocator
- [x] Fixed virtual area for kernel data
- [x] Cooperative multi-tasking for kernel tasks
    - [x] Naive executor
    - [x] Async-ify `FileInterface`
    - [x] Maybe Stream-ify `FileInterface`?
    - [x] Proper executor
- [x] Read from SDHC card
- [x] Print kernel argv
- [x] Switch to EL1 from EL2
- [x] Enable paging for EL1
- [x] CI with Docker + GH actions
- [ ] Dynamically sized virtual allocator for kernel data
    - [x] Dynamically map pages and allocate page tables
- [x] Exception handling
- [x] Interrupts
  - [ ] UART1 interrupts
  - [x] Timer interrupts
- [ ] Multicore
  - [x] Park cores properly
  - [ ] Execute tasks
- [x] Higher-half kernel
- [ ] Make use of DTB
- [ ] Parse tar initrd
- [ ] Run code in EL0 (usermode)
- [ ] Paging for usermode
- [ ] FAT32 driver
- [x] IPC layer (basic)
- [ ] VFS layer?
- [ ] Structured Exception Handling
- [ ] Simple Bluetooth
- [ ] Power management for RPI3
- [ ] USB
- [ ] USB HID Keyboard
- [ ] USB CDC Ethernet
- [ ] Usermode ICMP ping utility
