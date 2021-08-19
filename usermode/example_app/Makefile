OUT_DIR ?= .


${OUT_DIR}/example_app: main.c
	clang --target=aarch64-none-elf -O3 $< -o $@ -static -nostdlib -nostartfiles
	objcopy -O binary $@ $@.bin -j .text -j .rodata