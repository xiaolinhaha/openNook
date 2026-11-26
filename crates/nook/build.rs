// Embeds the icon SVGs (assets/icons, assets/agents as "agent-<name>") and the shipped mods
// (mods/<name>/...) as lists the binary carries.

use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

fn files(dir: &Path, prefix: &str, out: &mut Vec<(String, String)>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir).unwrap().flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let path = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            files(&path, &format!("{prefix}{name}/"), out);
        } else {
            out.push((format!("{prefix}{name}"), path.canonicalize().unwrap().to_string_lossy().into_owned()));
        }
    }
}

fn build_app_manager_probe(root: &Path, out_dir: &Path) -> Option<PathBuf> {
    if std::env::var("CARGO_CFG_TARGET_OS").unwrap() != "macos" {
        return None;
    }
    let arch = match std::env::var("CARGO_CFG_TARGET_ARCH").unwrap().as_str() {
        "aarch64" => "arm64",
        "x86_64" => "x86_64",
        arch => panic!("unsupported macOS architecture: {arch}"),
    };
    let probe = out_dir.join("app-manager-probe");
    let status = Command::new("xcrun")
        .args(["--sdk", "macosx", "swiftc", "-O", "-target"])
        .arg(format!("{arch}-apple-macosx14.0"))
        .arg(root.join("mods/app-manager/probe.swift"))
        .arg("-o")
        .arg(&probe)
        .status()
        .expect("run swiftc for the app manager probe");
    assert!(status.success(), "could not build the app manager probe");
    Some(probe)
}

fn main() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let out_dir = std::env::var("OUT_DIR").unwrap();
    let out_dir = Path::new(&out_dir);
    println!("cargo:rerun-if-changed=assets");
    println!("cargo:rerun-if-changed=mods");

    let mut icons = String::from("pub static ICON_FILES: &[(&str, &str)] = &[\n");
    for (dir, prefix) in [("assets/icons", ""), ("assets/agents", "agent-")] {
        let mut found = Vec::new();
        files(&root.join(dir), "", &mut found);
        for (name, path) in found {
            if let Some(stem) = name.strip_suffix(".svg") {
                writeln!(icons, "    ({:?}, include_str!({:?})),", format!("{prefix}{stem}"), path).unwrap();
            }
        }
    }
    icons.push_str("];\n");
    std::fs::write(out_dir.join("icons.rs"), icons).unwrap();

    let mut mods = String::from("pub static SHIPPED_MODS: &[(&str, &[u8])] = &[\n");
    let mut found = Vec::new();
    files(&root.join("mods"), "", &mut found);
    for (name, path) in found {
        if name == "app-manager/probe" {
            continue;
        }
        writeln!(mods, "    ({name:?}, include_bytes!({path:?})),").unwrap();
    }
    if let Some(probe) = build_app_manager_probe(root, out_dir) {
        writeln!(mods, "    (\"app-manager/probe\", include_bytes!({probe:?})),").unwrap();
    }
    mods.push_str("];\n");
    std::fs::write(out_dir.join("mods.rs"), mods).unwrap();
}
