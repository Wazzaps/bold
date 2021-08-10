# BoldOS

[![GitHub Workflow Status](https://img.shields.io/github/workflow/status/Wazzaps/bold/CI?style=flat-square)](https://github.com/yzhang-gh/vscode-markdown/actions)
[![GitHub stars](https://img.shields.io/github/stars/Wazzaps/bold.svg?style=flat-square&label=github%20stars)](https://github.com/yzhang-gh/vscode-markdown)
[![GitHub Contributors](https://img.shields.io/github/contributors/Wazzaps/bold.svg?style=flat-square)](https://github.com/yzhang-gh/vscode-markdown/graphs/contributors)

Tested on Raspberry pi 3 on QEMU

Mix of:

- `https://github.com/cs140e/rpi3-rust-template`
- `https://github.com/bztsrc/raspi3-tutorial`
- `https://wiki.osdev.org/Raspberry_Pi_Bare_Bones`
- My ideas
---

## Screenshot

![Screenshot](https://i.ibb.co/YQqhQHH/Screenshot-from-2021-07-18-01-43-15.png)

## Development environment (linux) - with GUI

- Install dependencies:
  - `apt install clang llvm binutils-aarch64-linux-gnu dosfstools mtools curl gdb-multiarch qemu-system-aarch64`
- Install rust:
  - `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
    - (Default everything)
  - `cargo install xargo`
  - `rustup component add rust-src`
  - `rustup override set nightly`
- Run it:
  - `xargo run --release`

## Development environment (linux) - without GUI

- Install dependencies:
  - `apt install clang llvm binutils-aarch64-linux-gnu dosfstools mtools curl gdb-multiarch`
  - `apt install --no-install-recommends qemu-system-aarch64`
- Install rust:
  - `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
    - (Default everything)
    - `source $HOME/.cargo/env`
  - `cargo install xargo`
  - `rustup override set nightly`
  - `rustup component add rust-src`
- Run it:
  - `xargo run --release -- -nographic -monitor none`

## Extra stuff

### GDB

- Run the kernel (either `xargo run --release` or `xargo run-stopped`)
- `./scripts/gdb_attach.sh`

### Parsing exceptions

- Copy the "ESR" value
- Run `parse_esr.py`, and paste it in

## Todo

- [x] Physical page allocator
- [x] Fixed virtual area for kernel data
- [ ] Cooperative multi-tasking for kernel tasks
    - [x] Naive executor
    - [x] Async-ify `FileInterface`
    - [x] Maybe Stream-ify `FileInterface`?
    - [ ] Proper executor
- [x] Read from SDHC card
- [x] Print kernel argv
- [x] Switch to EL1 from EL2
- [x] Enable paging for EL1
- [x] CI with Docker + GH actions
- [ ] Dynamically sized virtual allocator for kernel data
    - [x] Dynamically map pages and allocate page tables
- [x] Exception handling
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
