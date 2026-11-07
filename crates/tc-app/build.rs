//! Recursos de Windows de `parlo.exe` (icono PROVISIONAL, manifiesto, metadatos de versión). ADR-0006.
//! En Linux y macOS no hace nada: `winresource` solo es dependencia de compilación en Windows.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=../../packaging/windows/parlo.ico");
    println!("cargo:rerun-if-changed=../../packaging/windows/parlo.manifest");
    #[cfg(windows)]
    windows_resources();
}

#[cfg(windows)]
fn windows_resources() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../packaging/windows");
    let icon = root.join("parlo.ico");
    let manifest = root.join("parlo.manifest");
    let mut res = winresource::WindowsResource::new();
    res.set_icon(&icon.to_string_lossy())
        .set_manifest_file(&manifest.to_string_lossy())
        .set("ProductName", "Parlo")
        .set("FileDescription", "Parlo")
        .set("InternalName", "parlo")
        .set("OriginalFilename", "parlo.exe")
        .set("LegalCopyright", "MIT OR Apache-2.0");
    if let Err(e) = res.compile_for("parlo") {
        // Sin el compilador de recursos (rc.exe / windres) el binario funciona igual, sin icono ni manifiesto.
        println!("cargo:warning=no se pudieron incrustar los recursos de Windows: {e}");
    }
}
