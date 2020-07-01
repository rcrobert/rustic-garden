# rustic-garden
A garden irrigation controller for RPi written in Rust.

# Cross Compiling for Raspberry Pi
Configure target via `rustup` and download compiler.
```zsh
❯ rustup target add armv7-unknown-linux-gnueabihf

# Trying newer bionic gcc8 instead of 4.7
❯ sudo apt-get install gcc-8-multilib-arm-linux-gnueabihf
```
Define our target in `.cargo/config`
```toml
[target.armv7-unknown-linux-gnueabihf]
linker = "arm-linux-gnueabihf-gcc-8"
```
And build it! Our target name for this is defined from the config file above, `armv7-unknown-linux-gnueabihf`.
```zsh
❯ cargo build --target=armv7-unknown-linux-gnueabihf
   Compiling void v1.0.2
   Compiling cfg-if v0.1.10
   Compiling arc-swap v0.4.6
   Compiling libc v0.2.70
   Compiling bitflags v1.2.1
   Compiling nix v0.14.1
   Compiling signal-hook-registry v1.2.0
   Compiling signal-hook v0.1.15
   Compiling sysfs_gpio v0.5.4
   Compiling rustic-garden v0.1.0
    Finished dev [unoptimized + debuginfo] target(s) in 6.16s
❯ file target/armv7-unknown-linux-gnueabihf/debug/rustic-garden
target/armv7-unknown-linux-gnueabihf/debug/rustic-garden: ELF 32-bit LSB shared object, ARM, EABI5 version 1 (SYSV), dynamically linked, interpreter /lib/ld-linux-armhf.so.3, for GNU/Linux 3.2.0, BuildID[sha1]=922cc4abb4da8fb97e79717ceea273317c784d9e, with debug_info, not stripped
```

# Additional Resources
[Design visual](https://app.lucidchart.com/invitations/accept/6f6af9d6-f526-4acd-97d5-98f85f599d54)
