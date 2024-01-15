#!/bin/bash

# Needed to exit from script on error
set -e

buildargs=(
    "-p smartvaults-core"
    "-p smartvaults-core --features reserves"
    "-p smartvaults-core --target wasm32-unknown-unknown"
    "-p smartvaults-protocol"
    "-p smartvaults-protocol --target wasm32-unknown-unknown"
    "-p smartvaults-sdk"
    "-p smartvaults-cli"
    "-p smartvaults-desktop"
)

for arg in "${buildargs[@]}"; do
    echo  "Checking '$arg'"
    cargo check $arg
    if [[ $arg != *"--target wasm32-unknown-unknown"* ]]; then
        cargo $version test $arg
    fi
    cargo $version clippy $arg -- -D warnings
    echo
done