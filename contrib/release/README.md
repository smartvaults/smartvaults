# Release

## Build dependencies

### Ubuntu & Debian

#### To build arm64/aarch64 binaries from a x86_64 CPU

```bash
sudo dpkg --add-architecture arm64
sudo apt update
sudo apt install gcc-aarch64-linux-gnu g++-aarch64-linux-gnu libc6-dev:arm64
```

#### To build windows binaries

```bash
sudo apt install mingw-w64 wixl
```

### MacOS

#### To package DMG

```bash
brew install create-dmg
```