#!/usr/bin/env bash

set -e
KERNEL_ELF="$1"
BUILD_DIR="$(dirname "$KERNEL_ELF")"
TARGET="$(basename "$(dirname "$(dirname "$KERNEL_ELF")")")"

# Make initrd
INITRD_DIR="$BUILD_DIR/initrd"
INITRD="$BUILD_DIR/initrd.tar"
rm -rf "$INITRD_DIR" "$INITRD"
mkdir "$INITRD_DIR"
echo hello > "$INITRD_DIR"/hello
echo world > "$INITRD_DIR"/world
pushd "$INITRD_DIR"
tar -cf ../initrd.tar ./*
popd

# Make disk image
DISK_IMG="$BUILD_DIR/disk.img"
rm -f "$DISK_IMG"
fallocate "$DISK_IMG" -l 64MiB
mkfs.vfat -F 32 -n 'BOLD SYSTEM' "$DISK_IMG"
mcopy -i "$DISK_IMG" "$INITRD_DIR"/hello ::hello
mcopy -i "$DISK_IMG" "$INITRD_DIR"/world ::world

# Convert kernel to bin file
KERNEL_BIN="$KERNEL_ELF".bin
llvm-objcopy --input-target="$TARGET" "$KERNEL_ELF" --output-target binary "$KERNEL_BIN"
