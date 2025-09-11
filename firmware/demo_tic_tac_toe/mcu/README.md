
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

Then just run it as usual

```sh
cargo run --release
```

----

## Hardware

It is expected that a 16x16 neopixel matrix is connected on pin GPIO21.