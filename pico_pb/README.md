```bash
cargo flash --release --chip RP2040
```

```bash
cargo flash --manifest-path ../../bootloader/rp/Cargo.toml --release --chip RP2040
```

```bash
probe-rs attach --chip RP2040 .\target\thumbv6m-none-eabi\release\mdrc-pacbot-pico
```

```bash
cargo objcopy --release -- -O binary latest.bin
```