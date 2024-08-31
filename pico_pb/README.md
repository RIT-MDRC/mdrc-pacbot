To flash the program for the first time:
```bash
cargo flash --release --chip RP2040
```

In the Embassy project, use this after manual flashing to restore the bootloader:
```bash
cargo flash --manifest-path ../../bootloader/rp/Cargo.toml --release --chip RP2040
```

To attach a debugger:
```bash
probe-rs attach --chip RP2040 .\target\thumbv6m-none-eabi\release\mdrc-pacbot-pico
```

To prepare the project before uploading via Over the Air Programming:
```bash
cargo objcopy --release -- -O binary latest.bin
```