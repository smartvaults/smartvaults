// Copyright (c) 2022-2023 Coinstr
// Distributed under the MIT software license

fn main() {
    uniffi::generate_scaffolding("./src/coinstr_sdk.udl").expect("Building the UDL file failed");
}
