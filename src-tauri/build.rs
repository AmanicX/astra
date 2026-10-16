fn main() {
    tauri_build::build();

    // tauri-build attaches the Common-Controls v6 manifest resource only to
    // binary targets via `cargo:rustc-link-arg-bins`. Test harnesses are
    // separate executables that still import comctl32 v6 (TaskDialogIndirect),
    // so without the same resource the Windows loader aborts the process at
    // startup with STATUS_ENTRYPOINT_NOT_FOUND (0xC0000139). Attach it to every
    // link target (`-tests` does not apply to unit-test harnesses).
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        if let Ok(out) = std::env::var("OUT_DIR") {
            let resource = std::path::Path::new(&out).join("libresource.a");
            if resource.exists() {
                println!("cargo:rustc-link-arg={}", resource.display());
            }
        }
    }
}
