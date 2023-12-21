# BUILD FOR UNIX

## Introduction

Before build, see [build requirements](#build-requirements) for your specific platform.

## Build

### Both CLI and GUI

```
make
```

### GUI only

```
make gui
```

### CLI only

```
make cli
```

When build is completed, you can find `smartvaults-desktop` and/or `smartvaults-cli` binaries in `target/release` folder.

## Usage

Before using `smartvaults-desktop` or `smartvaults-cli`, take a look at [usage](./usage/README.md) guide.

## Build requirements

### Ubuntu & Debian

```
sudo apt install build-essential libusb-1.0-0-dev libudev-dev python3-dev
```

GUI dependencies:

```
sudo apt install build-essential pkg-config libclang-dev libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev libxcb-xfixes0-dev libspeechd-dev libxkbcommon-dev libssl-dev libdbus2.0-cil-dev
```

### Fedora

```
sudo dnf group install "C Development Tools and Libraries" "Development Tools"
```

GUI dependencies:

```
sudo dnf install clang clang-devel clang-tools-extra speech-dispatcher-devel libxkbcommon-devel pkg-config openssl-devel libxcb-devel gtk3-devel
```

### MacOS

```
xcode-select --install
```
