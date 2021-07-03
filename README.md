# BoldOS

Tested on Raspberry pi 3 on QEMU (`make run`)

Mix of:

- `https://github.com/cs140e/rpi3-rust-template`
- `https://github.com/bztsrc/raspi3-tutorial`
- `https://wiki.osdev.org/Raspberry_Pi_Bare_Bones`
- My ideas

## Todo

- [x] Physical page allocator
- [x] Fixed virtual area for kernel data
- [ ] Cooperative multi-tasking for kernel tasks
    - [x] Naive executor
    - [x] Async-ify `FileInterface`
    - [ ] Maybe Stream-ify `FileInterface`?
    - [ ] Proper executor
- [ ] Read from SDHC card
- [ ] Power management for RPI3
- [ ] Dynamic virtual areas for kernel data
- [ ] Exception handling
- [ ] USB
- [ ] USB HID Keyboard
- [ ] USB CDC Ethernet
- [ ] Usermode ICMP ping utility
