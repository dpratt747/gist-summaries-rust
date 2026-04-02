use std::path::PathBuf;

fn main() {
    let target = std::env::var("TAURI_ENV_TARGET_TRIPLE").unwrap_or_default();
    let sidecar_name = format!("ollama-{target}");
    let crate_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let sidecar_src = crate_dir.join("binaries").join(&sidecar_name);

    // Tauri validates that the externalBin source file exists at compile time.
    // Create a placeholder if the real binary hasn't been downloaded yet.
    if !sidecar_src.exists() {
        std::fs::write(&sidecar_src, "stub").ok();
    }

    tauri_build::build()
}
