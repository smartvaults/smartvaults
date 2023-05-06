# Coinstr Core FFI

## Prerequisites
* When building for Android:
  * Set the ANDROID_NDK_HOME env variable to your sdk home folder

## Build

On first usage you will need to run:

```
make init
```

### Kotlin

### Libraries and Bindings

This command will build libraries for different platforms in `target/` folder and copy them to `ffi/kotlin/jniLibs`.
In addition it will generate Kotlin bindings in `ffi/kotlin/coinstr`.

```
make kotlin
```

## License

This project is distributed under the MIT software license - see the [LICENSE](../../LICENSE) file for details
