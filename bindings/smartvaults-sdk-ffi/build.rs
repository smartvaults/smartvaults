// Copyright (c) 2022-2023 Smart Vaults
// Distributed under the MIT software license

use std::env;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use std::ops::Add;
use std::path::Path;

const DEFAULT_CLANG_VERSION: &str = "14.0.7";
const OUTPUT_UDL_NAME: &str = "common";

fn main() {
    println!("cargo:rerun-if-changed=src/*.udl");

    setup_x86_64_android_workaround();
    merge_udl_files();

    uniffi::generate_scaffolding(format!("src/{OUTPUT_UDL_NAME}.udl"))
        .expect("Building the UDL file failed");
}

/// Adds a temporary workaround for an issue with the Rust compiler and Android
/// in x86_64 devices: https://github.com/rust-lang/rust/issues/109717.
/// The workaround comes from: https://github.com/mozilla/application-services/pull/5442
fn setup_x86_64_android_workaround() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").expect("CARGO_CFG_TARGET_OS not set");
    let target_arch = env::var("CARGO_CFG_TARGET_ARCH").expect("CARGO_CFG_TARGET_ARCH not set");
    if target_arch == "x86_64" && target_os == "android" {
        let android_ndk_home = env::var("ANDROID_NDK_HOME").expect("ANDROID_NDK_HOME not set");
        let build_os = match env::consts::OS {
            "linux" => "linux",
            "macos" => "darwin",
            "windows" => "windows",
            _ => panic!(
                "Unsupported OS. You must use either Linux, MacOS or Windows to build the crate."
            ),
        };
        let clang_version =
            env::var("NDK_CLANG_VERSION").unwrap_or_else(|_| DEFAULT_CLANG_VERSION.to_owned());
        let linux_x86_64_lib_dir = format!(
            "toolchains/llvm/prebuilt/{build_os}-x86_64/lib64/clang/{clang_version}/lib/linux/"
        );
        let linkpath = format!("{android_ndk_home}/{linux_x86_64_lib_dir}");
        if Path::new(&linkpath).exists() {
            println!("cargo:rustc-link-search={android_ndk_home}/{linux_x86_64_lib_dir}");
            println!("cargo:rustc-link-lib=static=clang_rt.builtins-x86_64-android");
        } else {
            panic!("Path {linkpath} not exists");
        }
    }
}

fn merge_udl_files() {
    let path = format!("src/{}.udl", OUTPUT_UDL_NAME);
    let mut file = File::create(&path).unwrap_or_else(|_| panic!("Could not create {}", path));

    let header_comment = r#"// This file is auto-generated and contains the concatenated contents of all the UDL files in src directory
// Do not edit this manually but instead editing/adding UDL files in the src directory

"#;

    file.write_all(header_comment.as_bytes())
        .unwrap_or_else(|_| panic!("Error writing to {path}"));

    let other_udl_file_paths =
        scan_udl_files().unwrap_or_else(|_| panic!("Can not scan src directory"));

    for path in other_udl_file_paths {
        merge(path, &file);
    }
}

// Get all the UDL files excluding OUTPUT_UDL_NAME in src directory
fn scan_udl_files() -> Result<Vec<String>, io::Error> {
    let files = fs::read_dir("src")?;
    let udl_files: Vec<_> = files
        .filter_map(|file| {
            let file = file.ok()?;
            let path = file.path();
            if path.is_file() && path.extension().map_or(false, |ext| ext == "udl") {
                match path.file_name() {
                    Some(file_name) => match file_name.to_str() {
                        Some(value) => {
                            if value == format!("{OUTPUT_UDL_NAME}.udl") {
                                None
                            } else {
                                Some(format!("src/{value}"))
                            }
                        }
                        None => None,
                    },
                    None => None,
                }
            } else {
                None
            }
        })
        .collect();

    Ok(udl_files)
}

fn merge(src_udl_path: String, mut file: &File) {
    let content = get_string_from_file_path(&src_udl_path)
        .unwrap_or_else(|_| panic!("Fail to read the content of {}", src_udl_path))
        .add("\n\n");
    file.write_all(content.as_bytes())
        .unwrap_or_else(|_| panic!("Error copying from {}", src_udl_path));
}

fn get_string_from_file_path(path: &str) -> Result<String, io::Error> {
    let mut file = File::open(path)?;
    let mut content = String::new();
    file.read_to_string(&mut content)?;
    Ok(content)
}
