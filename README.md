# longan-nano-playground.rs

Longan Nano board examples, in Rust, under macOS. Bad Apple included.

A playground with [riscv-rust/longan-nano](https://github.com/riscv-rust/longan-nano).

## Environment setup

```console
$ # Download RISC-V gcc toolchains to PATH (both rv32 and rv64 will do)
$ riscv64-unknown-elf-objcopy --version
GNU objcopy (SiFive Binutils 2.32.0-2020.04.0) 2.32
...

$ # Install dfu-tool from brew (git HEAD required, including bugfix for GD32VF)
$ brew install dfu-util --HEAD

$ # List device (Hold BOOT0 and Press RESET, device reboot to DFU mode)
$ dfu-util -l
Found DFU: [28e9:0189] ver=0100, devnum=6, cfg=1, intf=0, path="20-1", alt=0, name="@Internal Flash  /0x08000000/128*001Kg", serial="3CBJ"

$ # Add rust toolchain for riscv32imac
$ rustup target add riscv32imac-unknown-none-elf
```

Run on Longan Nano board:

```sh
# cargo build..
./build.sh

# Hold BOOT0 and Press RESET. reboot to DFU mode...
./flash.sh

# Press RESET on the board
```

## A writter for byte array

You cannot use `println!()` and `writeln!()` in a bare embedded device, since no alloc lib defined.
And `String`, `Vec<u8>` requires dynamic allocation on heap.

So a `ByteMutWriter` is defined for `core::fmt::Write`.

Usage:

```rust
let mut buf = [0u8; 20 * 5];
let mut buf = ByteMutWriter::new(&mut buf[..]);

buf.clear();

writeln!(buf, "Hello {}", "Rust");
write!(buf, "Val: 0x{:08x}", debug_val);

// buf.as_str();
```

## Steps for Bad Apple

- Download a Bad Apple video
- Convert video file to image sequences (scale to LCD screen resolution)
- Convert images to a `Rgb565` ImageRaw file

```sh
# Download video
youtube-dl https://....

# Video info
ffmpeg -i BadApple.mp4
# ....
# Stream #0:0(und): Video: h264 (High) (avc1 / 0x31637661), yuv420p(tv, bt709), 960x720 [SAR 1:1 DAR 4:3], 567 kb/s, 30 fps, 30 tbr, 15360 tbn, 60 tbc (default)
# ....

# Now you know it's a 960x720 resolution, 30 fps.
# To scale it to fit Longan Nano's 160x80 screen, you will need to scale it to 106x80.

# NOTE: Lower fps might help.

# Now, convert to image sequences.
ffmpeg -i BadApple.mp4 -vf scale=106:80,fps=24 'out/%04d.png'

# Convert the image sequence to a `Rgb565` ImageRaw file. (requires python3-pillow)
python3 scripts/convert.py
```

Then copy the `badapple.raw` to your SD card.
