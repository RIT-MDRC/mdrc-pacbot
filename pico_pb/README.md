To flash the program for the first time:

```bash
cargo flash --release --chip RP2040
```

If the flash was overwritten, also flash the cyw43 firmware:

```bash
probe-rs download cyw43-firmware/43439A0.bin --binary-format bin --chip RP2040 --base-address 0x101B0000
probe-rs download cyw43-firmware/43439A0_clm.bin --binary-format bin --chip RP2040 --base-address 0x101F0000
```

To flash the bootloader:

```bash
cargo flash --manifest-path ./boot/rp/Cargo.toml --release --chip RP2040
```

To attach a debugger:

```bash
probe-rs attach --chip RP2040 .\target\thumbv6m-none-eabi\release\mdrc-pacbot-pico
```

To prepare the project before uploading via Over the Air Programming:

```bash
cargo objcopy --release -- -O binary latest.bin
```