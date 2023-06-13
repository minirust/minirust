use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=./intrinsics");
    Command::new("cargo")
            .args(["b", "--manifest-path", "./intrinsics/Cargo.toml"])
            .output()
            .expect("failed to compile `intrinsics`");
}
