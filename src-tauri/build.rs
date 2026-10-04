fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap();
    for file in [
        "scripts/build-helper.mjs",
        "vscode-helper/package.json",
        "vscode-helper/extension.cjs",
        "vscode-helper/startup.cjs",
        "vscode-helper/README.md",
    ] {
        println!("cargo:rerun-if-changed={}", root.join(file).display());
    }
    let status = std::process::Command::new("node")
        .arg(root.join("scripts/build-helper.mjs"))
        .status()
        .expect("Node.js is required to package the RepoJump companion extension");
    assert!(status.success(), "Companion extension packaging failed");
    tauri_build::build()
}
