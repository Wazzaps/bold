#!/usr/bin/env bash

TARGET="aarch64-none-elf"
# GDB_PATH="/opt/compilers/gcc-arm-10.2-2020.11-x86_64-aarch64-none-elf/bin/aarch64-none-elf-gdb"
GDB_PATH="gdb-multiarch"
GDB_PORT=1234

exec "$GDB_PATH" -ex "target remote :$GDB_PORT" target/"$TARGET"/release/boldos
