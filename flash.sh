#!/bin/sh

set -e

riscv64-unknown-elf-objcopy -O binary \
    target/riscv32imac-unknown-none-elf/release/examples/bad_apple \
    firmware.bin


dfu-util -a 0 -s 0x08000000:leave -D firmware.bin
