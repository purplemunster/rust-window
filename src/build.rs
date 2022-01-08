// build.rs

use std::env;
use std::path::Path;
use std::process::Command;

fn main() {
    let src_dir = env::current_dir().unwrap();
    let src_path = Path::new(&src_dir).join("src/shaders.metal");

    let out_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/");
    let dst_path = out_dir.to_str().unwrap();

    println!("Building shaders in {}", dst_path);

    Command::new("xcrun").args(&["-sdk", "macosx", "metal", "-c"])
                         .arg(&format!("{}", src_path.to_str().unwrap()))
                         .arg("-o")
                         .arg(&format!("{}/shaders.air", dst_path))
                         .status().expect("Failed to compile shaders!");

    Command::new("xcrun").args(&["-sdk", "macosx", "metallib"])
                         .arg(&format!("{}/shaders.air", dst_path))
                         .arg("-o")
                         .arg(&format!("{}/shaders.metallib", dst_path))
                         .status().expect("Failed to compile metal shaders!");

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=shaders.metal");
}