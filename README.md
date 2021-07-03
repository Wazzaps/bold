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
- [x] Read from SDHC card
- [x] Print kernel argv
- [ ] Parse tar initrd
- [ ] Run code in EL0 (usermode)
- [ ] Paging for usermode
- [ ] FAT32 driver
- [ ] IPC layer
- [ ] VFS layer?
- [ ] Simple Bluetooth
- [ ] Power management for RPI3
- [ ] Dynamically sized virtual allocator for kernel data
- [ ] Exception handling
- [ ] USB
- [ ] USB HID Keyboard
- [ ] USB CDC Ethernet
- [ ] Usermode ICMP ping utility
