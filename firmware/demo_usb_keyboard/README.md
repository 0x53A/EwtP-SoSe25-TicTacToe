
First, install rust, espup (for the xtensa toolchain) and 

```sh
# one time
cargo install espup --locked
espup install

# for each new shell,
#  only required for linux
source $HOME/export-esp.sh
```

----------

This program needs to compile the C-library "tinyusb" and therefore it's easiest to compile it under Linux (including WSL).

```sh
# set path to your xtensa toolchain
export XTENSA_TOOLCHAIN=/home/lukas/.rustup/toolchains/esp/xtensa-esp-elf/esp-14.2.0_20240906/xtensa-esp-elf/bin

# tell the cc crate to use the cross-compiler for that target
export CC_xtensa_esp32s3_none_elf="$XTENSA_TOOLCHAIN/xtensa-esp32s3-elf-gcc"
export AR_xtensa_esp32s3_none_elf="$XTENSA_TOOLCHAIN/xtensa-esp32s3-elf-ar"

# tell cargo which linker to invoke for the target
export CARGO_TARGET_XTENSA_ESP32S3_NONE_ELF_LINKER="$CC_xtensa_esp32s3_none_elf"
```

```sh
export BINDGEN_EXTRA_CLANG_ARGS="-I/home/lukas/.rustup/toolchains/esp/xtensa-esp-elf/esp-14.2.0_20240906/xtensa-esp-elf/xtensa-esp-elf/include"

export LIBCLANG_PATH=/home/lukas/.rustup/toolchains/esp/xtensa-esp32-elf-clang/esp-19.1.2_20250225/esp-clang/lib
```


------

When running natively under Linux, you can just run the program

```sh
cargo run --release
```

When running under Windows/WSL, you can either compile in WSL and flash in windows ...

```sh
# in WSL
cargo build --release

# in a different shell, in Windows
espflash flash --monitor --partition-table=partitions.csv target/xtensa-esp32s3-none-elf/release/esp32_keyboard_demo
```


or you can forward the USB device to WSL and do everything, including debugging, in WSL:

https://learn.microsoft.com/en-us/windows/wsl/connect-usb

```sh
usbipd list
usbipd bind --busid 1-5
usbipd attach --wsl --busid 1-5
```

You'll need to add the udev rules:

```sh
sudo nano /etc/udev/rules.d/99-serial-acm.rules
```

```
SUBSYSTEM=="tty", ATTRS{idVendor}=="1a86", ATTRS{idProduct}=="55d3", MODE="0666"
```

```sh
sudo udevadm control --reload-rules
sudo udevadm trigger
```