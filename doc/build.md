# BUILD

## Download source code

```
git clone https://github.com/coinstr/coinstr.git && cd coinstr
```

## Verify commits

Import gpg keys:

```
gpg --keyserver hkps://keys.openpgp.org --recv-keys $(<contrib/verify-commits/trusted-keys)
```

Verify commit:

```
git verify-commit HEAD
```

## Install Rust

Follow this instructions: https://www.rust-lang.org/tools/install

## Build

Follow instruction for your OS:

* [Unix](build-unix.md) 

## Usage

Before using `coinstr` or `coinstr-cli`, take a look at [usage](./usage/README.md) guide.
