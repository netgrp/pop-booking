use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir =
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR missing"));
    let workspace_root = manifest_dir
        .parent()
        .expect("backend must live inside workspace")
        .to_path_buf();
    let snow_crate = workspace_root.join("frontend/snow-wasm");
    let snow_src = snow_crate.join("src");

    println!(
        "cargo:rerun-if-changed={}",
        snow_crate.join("Cargo.toml").display()
    );
    println!("cargo:rerun-if-changed={}", snow_src.display());
    println!("cargo:rerun-if-changed={}", snow_crate.join("pkg").display());
    println!("cargo:rerun-if-env-changed=WASM_BINDGEN");

    let skip_wasm_build = env::var("CI").is_ok() || env::var("SKIP_SNOW_WASM_BUILD").is_ok();
    if skip_wasm_build {
        let pkg_dir = snow_crate.join("pkg");
        let wasm = pkg_dir.join("snow_sim_bg.wasm");
        let js = pkg_dir.join("snow_sim.js");
        if wasm.is_file() && js.is_file() {
            println!(
                "cargo:warning=Skipping snow_wasm build; using prebuilt artifacts in {}",
                pkg_dir.display()
            );
            return;
        }
        panic!(
            "CI skip requested but prebuilt wasm bundle is missing (expected {} and {})",
            wasm.display(),
            js.display()
        );
    }

    let cargo = env::var("CARGO").unwrap_or_else(|_| "cargo".to_string());
    let base_rustflags = env::var("RUSTFLAGS").unwrap_or_default();
    let mut rustflags = base_rustflags.trim().to_string();
    if !rustflags.is_empty() {
        rustflags.push(' ');
    }
    rustflags.push_str("-C opt-level=3 -C codegen-units=1 -C lto=fat");

    let default_target_dir = env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| workspace_root.join("target"));
    let wasm_target_dir = default_target_dir.join("wasm-cache");

    let build_status = Command::new(&cargo)
        .current_dir(&workspace_root)
        .args([
            "build",
            "--package",
            "snow_wasm",
            "--release",
            "--target",
            "wasm32-unknown-unknown",
        ])
        .env("RUSTFLAGS", &rustflags)
        .env("CARGO_TARGET_DIR", &wasm_target_dir)
        .status()
        .expect("failed to invoke cargo for snow_wasm");

    if !build_status.success() {
        panic!("building snow_wasm failed");
    }

    let target_wasm = wasm_target_dir.join("wasm32-unknown-unknown/release/snow_wasm.wasm");
    if !target_wasm.exists() {
        panic!("expected wasm artifact at {}", target_wasm.display());
    }

    let pkg_dir = snow_crate.join("pkg");
    if pkg_dir.exists() {
        fs::remove_dir_all(&pkg_dir).expect("unable to clear previous pkg dir");
    }
    fs::create_dir_all(&pkg_dir).expect("unable to create pkg directory");

    let wasm_bindgen = env::var("WASM_BINDGEN").unwrap_or_else(|_| "wasm-bindgen".to_string());
    let bindgen_status = Command::new(&wasm_bindgen)
        .arg("--target")
        .arg("web")
        .arg("--out-dir")
        .arg(&pkg_dir)
        .arg("--out-name")
        .arg("snow_sim")
        .arg(&target_wasm)
        .status()
        .unwrap_or_else(|err| {
            panic!(
                "failed to run {} (install via `cargo install wasm-bindgen-cli` or set WASM_BINDGEN): {err}",
                wasm_bindgen
            )
        });

    if !bindgen_status.success() {
        panic!("wasm-bindgen emitted a non-zero exit status");
    }
}
