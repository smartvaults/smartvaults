// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

fn main() {
    prost_build::compile_protos(
        &["src/v2/proto/vault.proto", "src/v2/proto/wrapper.proto"],
        &["src/v2/proto"],
    )
    .unwrap();
}
