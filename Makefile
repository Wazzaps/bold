TARGET ?= aarch64-none-elf
CROSS ?= $(TARGET)

CC := $(CROSS)-gcc
CCFLAGS ?= -Wall -O2 -nostdlib -nostartfiles -ffreestanding -pie
XARGO ?= CARGO_INCREMENTAL=0 RUST_TARGET_PATH="$(shell pwd)" xargo

LD_SCRIPT := src/arch/aarch64/build/linker.ld

RUST_BINARY := $(shell cat Cargo.toml | grep name | cut -d\" -f 2 | tr - _)
RUST_BUILD_DIR := target/$(TARGET)
RUST_DEBUG_LIB := $(RUST_BUILD_DIR)/debug/lib$(RUST_BINARY).a
RUST_RELEASE_LIB := $(RUST_BUILD_DIR)/release/lib$(RUST_BINARY).a

RUST_DEPS = Xargo.toml Cargo.toml build.rs $(LD_SCRIPT) $(shell find src/ -type f -name '*.rs')
EXT_DEPS = $(BUILD_DIR)/init.o

BUILD_DIR := build
KERNEL := $(BUILD_DIR)/$(RUST_BINARY)
RUST_LIB := $(BUILD_DIR)/$(RUST_BINARY).a

.PHONY: all clean check

VPATH = src/arch/aarch64/build

all: $(KERNEL).hex $(KERNEL).bin

check:
	@$(XARGO) check --target=$(TARGET)

$(RUST_DEBUG_LIB): $(RUST_DEPS)
	@echo "+ Building $@ [xargo]"
	@$(XARGO) build --target=$(TARGET)

$(RUST_RELEASE_LIB): $(RUST_DEPS)
	@echo "+ Building $@ [xargo --release]"
	@$(XARGO) build --release --target=$(TARGET)

ifeq ($(DEBUG),1)
$(RUST_LIB): $(RUST_DEBUG_LIB) | $(BUILD_DIR)
	@cp $< $@
else
$(RUST_LIB): $(RUST_RELEASE_LIB) | $(BUILD_DIR)
	@cp $< $@
endif

$(BUILD_DIR):
	@mkdir -p $@

$(BUILD_DIR)/%.o: %.c | $(BUILD_DIR)
	@echo "+ Building $@ [cc $<]"
	@$(CC) $(CCFLAGS) -c $< -o $@

$(BUILD_DIR)/%.o: %.S | $(BUILD_DIR)
	@echo "+ Building $@ [as $<]"
	@$(CC) $(CCFLAGS) -c $< -o $@

$(KERNEL).elf: $(EXT_DEPS) $(RUST_LIB) | $(BUILD_DIR)
	@echo "+ Building $@ [ld $^]"
	@$(CROSS)-ld --gc-sections -o $@ $^ -T$(LD_SCRIPT)

$(KERNEL).hex: $(KERNEL).elf | $(BUILD_DIR)
	@echo "+ Building $@ [objcopy $<]"
	@$(CROSS)-objcopy $< -O ihex $@

$(KERNEL).bin: $(KERNEL).elf | $(BUILD_DIR)
	@echo "+ Building $@ [objcopy $<]"
	@$(CROSS)-objcopy $< -O binary $@

clean:
	$(XARGO) clean
	rm -rf $(BUILD_DIR)

run: $(KERNEL).bin
	qemu-system-aarch64 -M raspi3 -serial stdio -semihosting -kernel $(KERNEL).bin -s

qemugdb-run: $(KERNEL).bin
	gdb -ex=r --args qemu-system-aarch64 -M raspi3 -serial stdio -kernel $(KERNEL).bin

gdb:
	/opt/compilers/gcc-arm-10.2-2020.11-x86_64-aarch64-none-elf/bin/aarch64-none-elf-gdb -ex 'target remote :1234' $(KERNEL).elf