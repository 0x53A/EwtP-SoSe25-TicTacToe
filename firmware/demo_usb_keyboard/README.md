


```
cargo install espup --locked
espup install

cargo  run
```

----------

```
export BINDGEN_EXTRA_CLANG_ARGS="-I/home/lukas/.rustup/toolchains/esp/xtensa-esp-elf/esp-14.2.0_20240906/xtensa-esp-elf/xtensa-esp-elf/include"

export LIBCLANG_PATH=/home/lukas/.rustup/toolchains/esp/xtensa-esp32-elf-clang/esp-19.1.2_20250225/esp-clang/lib

cargo build --release
```