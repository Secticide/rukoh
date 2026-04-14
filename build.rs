use std::{env, path::PathBuf};

fn main() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let fxc = find_fxc();

    compile(
        &fxc,
        "src/shaders/sprite.hlsl",
        "vs_4_0",
        "vs_main",
        out_dir.join("sprite_vs.dxbc"),
    );
    compile(
        &fxc,
        "src/shaders/sprite.hlsl",
        "ps_4_0",
        "ps_main",
        out_dir.join("sprite_ps.dxbc"),
    );

    println!("cargo:rerun-if-changed=src/shaders/sprite.hlsl");
    println!("cargo:rerun-if-changed=build.rs");
}

fn compile(fxc: &PathBuf, src: &str, profile: &str, entry: &str, out: PathBuf) {
    let status = std::process::Command::new(fxc)
        .args(["/nologo", "/T", profile, "/E", entry, "/Fo"])
        .arg(&out)
        .arg(src)
        .status()
        .unwrap_or_else(|e| panic!("Failed to launch fxc.exe ({fxc:?}): {e}"));

    assert!(
        status.success(),
        "Shader compilation failed — src={src} entry={entry} profile={profile}"
    );
}

fn find_fxc() -> PathBuf {
    // 1. fxc.exe already on PATH (e.g. developer command prompt)
    if let Ok(out) = std::process::Command::new("where").arg("fxc.exe").output() {
        if out.status.success() {
            if let Some(line) = String::from_utf8_lossy(&out.stdout).lines().next() {
                let p = PathBuf::from(line.trim());
                if p.exists() {
                    return p;
                }
            }
        }
    }

    // 2. Search Windows SDK bin directories (newest version first)
    let sdk_bin = PathBuf::from(r"C:\Program Files (x86)\Windows Kits\10\bin");
    if sdk_bin.exists() {
        let mut versions: Vec<PathBuf> = std::fs::read_dir(&sdk_bin)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .collect();
        versions.sort_by(|a, b| b.cmp(a)); // descending = newest first
        for version in versions {
            let fxc = version.join("x64").join("fxc.exe");
            if fxc.exists() {
                return fxc;
            }
        }
    }

    panic!(
        "fxc.exe not found.\n\
         Either install the Windows SDK or add fxc.exe to your PATH.\n\
         Download: https://developer.microsoft.com/windows/downloads/windows-sdk/"
    );
}
