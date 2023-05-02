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

When build is completed, you can find `coinstr` and/or `coinstr-cli` binaries in `target/release` folder.

## Usage

Before using `coinstr` or `coinstr-cli`, take a look at [usage](./usage/README.md) guide.

## Build requirements

### Ubuntu & Debian

```
sudo apt install build-essential 
```

GUI dependencies:

TODO

### Fedora

```
sudo dnf group install "C Development Tools and Libraries" "Development Tools"
```

GUI dependencies:

TODO
