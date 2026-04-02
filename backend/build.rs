use std::path::PathBuf;

fn main() {
    let target = std::env::var("TAURI_ENV_TARGET_TRIPLE").unwrap_or_default();
    let sidecar_name = if cfg!(windows) && !target.is_empty() {
        format!("ollama-{target}.exe")
    } else {
        format!("ollama-{target}")
    };
    let crate_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let sidecar_src = crate_dir.join(&sidecar_name);

    // Tauri validates that the externalBin source file exists at compile time.
    // Create a placeholder if the real binary hasn't been downloaded yet.
    // On Windows in CI, CARGO_MANIFEST_DIR may resolve through a symlink —
    // find the real directory where the binary was downloaded.
    if !sidecar_src.exists() {
        if let Ok(canonical) = crate_dir.canonicalize() {
            let real_src = canonical.join(&sidecar_name);
            if real_src.exists() {
                // Copy from the canonical path to the symlinked path
                std::fs::copy(&real_src, &sidecar_src).ok();
            }
        }
    }
    if !sidecar_src.exists() {
        std::fs::write(&sidecar_src, "stub").ok();
    }

    tauri_build::build()
}
