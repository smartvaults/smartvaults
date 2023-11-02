CURRENT_DIR := $(shell pwd)
SOURCES=$(sort $(wildcard ./src/*.rs ./src/**/*.rs))

.PHONY: init
init:
	rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-ios-sim
	rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android
	@if [ $$(uname) == "Darwin" ] ; then cargo install cargo-lipo ; fi
	cargo install cbindgen
	cargo install cargo-ndk

.PHONY: ndk-home
ndk-home:
	@if [ ! -d "${ANDROID_NDK_HOME}" ] ; then \
		echo "Error: Please, set the ANDROID_NDK_HOME env variable to point to your NDK folder" ; \
		exit 1 ; \
	fi

.PHONY: sdk-root
sdk-root:
	@if [ ! -d "${ANDROID_SDK_ROOT}" ] ; then \
		echo "Error: Please, set the ANDROID_SDK_ROOT env variable to point to your SDK folder" ; \
		exit 1 ; \
	fi

clean-android:
	rm -rf ffi/android
	rm -rf ffi/kotlin

kotlin: clean-android android
	cargo run --features=uniffi/cli --bin uniffi-bindgen generate src/common.udl --language kotlin --no-format -o ffi/kotlin

android: aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android

aarch64-linux-android: $(SOURCES) ndk-home
	cargo ndk -t aarch64-linux-android -o ffi/kotlin/jniLibs build --release	

armv7-linux-androideabi: $(SOURCES) ndk-home
	cargo ndk -t armv7-linux-androideabi -o ffi/kotlin/jniLibs build --release	

i686-linux-android: $(SOURCES) ndk-home
	cargo ndk -t i686-linux-android -o ffi/kotlin/jniLibs build --release	

x86_64-linux-android: $(SOURCES) ndk-home
	cargo ndk -t x86_64-linux-android -o ffi/kotlin/jniLibs build --release

bindings-android: sdk-root kotlin
	rm -rf bindings-android/lib/src/main/jniLibs
	rm -rf bindings-android/lib/src/main/kotlin
	cp -r ffi/kotlin/jniLibs bindings-android/lib/src/main
	cp -r ffi/kotlin/io bindings-android/lib/src/main/kotlin/
	cd bindings-android && ./gradlew assemble
	mkdir -p ffi/android
	cp bindings-android/lib/build/outputs/aar/lib-release.aar ffi/android

publish-android: bindings-android
	cd bindings-android && ./gradlew publishToSonatype closeAndReleaseSonatypeStagingRepository

ios-universal: $(SOURCES)		
	mkdir -p ../../target/ios-universal/release
	mkdir -p ../../target/ios-universal-sim/release
	cargo build --release --target aarch64-apple-ios
	cargo build --release --target x86_64-apple-ios
	cargo build --release --target aarch64-apple-ios-sim
	lipo -create -output ../../target/ios-universal/release/libsmartvaults_sdk_ffi.a ../../target/aarch64-apple-ios/release/libsmartvaults_sdk_ffi.a ../../target/x86_64-apple-ios/release/libsmartvaults_sdk_ffi.a
	lipo -create -output ../../target/ios-universal-sim/release/libsmartvaults_sdk_ffi.a ../../target/aarch64-apple-ios-sim/release/libsmartvaults_sdk_ffi.a ../../target/x86_64-apple-ios/release/libsmartvaults_sdk_ffi.a

swift-ios: ios-universal
	cargo run --features=uniffi/cli --bin uniffi-bindgen generate src/common.udl -l swift -o ffi/swift-ios
	cp ../../target/ios-universal/release/libsmartvaults_sdk_ffi.a ffi/swift-ios
	cd ffi/swift-ios && "swiftc" "-emit-module" "-module-name" "smartvaults_sdk_ffi"  "-Xcc" "-fmodule-map-file=$(CURRENT_DIR)/ffi/swift-ios/smartvaults_sdkFFI.modulemap" "-I" "."  "-L" "." "-lsmartvaults_sdk_ffi" smartvaults_sdk.swift

bindings-swift: ios-universal
	mkdir -p bindings-swift/Sources/SmartVaultsSDK
	cargo run --features=uniffi/cli --bin uniffi-bindgen generate src/common.udl --no-format --language swift --out-dir bindings-swift/Sources/SmartVaultsSDK
	mv bindings-swift/Sources/SmartVaultsSDK/smartvaults_sdk.swift bindings-swift/Sources/SmartVaultsSDK/SmartVaultsSDK.swift
	cp bindings-swift/Sources/SmartVaultsSDK/smartvaults_sdkFFI.h bindings-swift/smartvaults_sdkFFI.xcframework/ios-arm64/smartvaults_sdkFFI.framework/Headers
	cp bindings-swift/Sources/SmartVaultsSDK/smartvaults_sdkFFI.h bindings-swift/smartvaults_sdkFFI.xcframework/ios-arm64_x86_64-simulator/smartvaults_sdkFFI.framework/Headers
	cp ../../target/aarch64-apple-ios/release/libsmartvaults_sdk_ffi.a bindings-swift/smartvaults_sdkFFI.xcframework/ios-arm64/smartvaults_sdkFFI.framework/smartvaults_sdkFFI
	cp ../../target/ios-universal-sim/release/libsmartvaults_sdk_ffi.a bindings-swift/smartvaults_sdkFFI.xcframework/ios-arm64_x86_64-simulator/smartvaults_sdkFFI.framework/smartvaults_sdkFFI
	rm bindings-swift/Sources/SmartVaultsSDK/smartvaults_sdkFFI.h
	rm bindings-swift/Sources/SmartVaultsSDK/smartvaults_sdkFFI.modulemap

python:
	rm -rf bindings-python/dist
	#pip install -r bindings-python/requirements.txt
	cargo build --release
	cargo run --features=uniffi/cli --bin uniffi-bindgen generate src/common.udl --language python --no-format -o bindings-python/src/smartvaults-sdk/
	cp ../../target/release/libsmartvaults_sdk_ffi.so bindings-python/src/smartvaults-sdk/ | true
	cp ../../target/release/libsmartvaults_sdk_ffi.dylib bindings-python/src/smartvaults-sdk/ | true
	cd bindings-python && python setup.py --verbose bdist_wheel
	pip install ./bindings-python/dist/smartvaults_sdk-*.whl --force-reinstall --break-system-packages
