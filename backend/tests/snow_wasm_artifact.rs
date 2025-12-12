use std::{collections::HashSet, fs, path::PathBuf};

use wasmparser::{Parser, Payload, Validator};

fn wasm_pkg_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("backend manifest must have a parent directory")
        .join("frontend")
        .join("snow-wasm")
        .join("pkg")
}

fn wasm_and_js_paths() -> (PathBuf, PathBuf) {
    let pkg_dir = wasm_pkg_dir();
    (
        pkg_dir.join("snow_sim_bg.wasm"),
        pkg_dir.join("snow_sim.js"),
    )
}

#[test]
fn snow_wasm_artifact_is_present_and_exports_expected_symbols() {
    let (wasm_path, _) = wasm_and_js_paths();
    assert!(
        wasm_path.is_file(),
        "Missing wasm artifact at {}",
        wasm_path.display()
    );

    let wasm_bytes = fs::read(&wasm_path)
        .unwrap_or_else(|err| panic!("Failed to read {}: {err}", wasm_path.display()));

    // Validate the module structure and collect its export names.
    let mut validator = Validator::new();
    let mut exports = HashSet::new();
    for payload in Parser::new(0).parse_all(&wasm_bytes) {
        let payload = payload.expect("wasm payload parsing failed");
        validator
            .payload(&payload)
            .expect("wasm module failed validation");

        if let Payload::ExportSection(section) = payload {
            for export in section {
                let export = export.expect("failed to parse export entry");
                exports.insert(export.name.to_owned());
            }
        }
    }

    let expected = [
        "snow_init",
        "snow_resize",
        "snow_step",
        "snow_pointer_move",
        "snow_pointer_wind",
        "snow_dynamic_count",
        "snow_inactive_count",
        "snow_pile_bins",
        "snow_reset",
        "snow_set_tint",
    ];

    let missing: Vec<_> = expected
        .iter()
        .filter(|name| !exports.contains(**name))
        .copied()
        .collect();

    assert!(
        missing.is_empty(),
        "WASM missing expected exports: {:?}",
        missing
    );
}

#[test]
fn snow_wasm_js_glue_targets_embedded_wasm() {
    let (_, js_path) = wasm_and_js_paths();
    assert!(
        js_path.is_file(),
        "Missing JS glue at {}",
        js_path.display()
    );

    let js = fs::read_to_string(&js_path)
        .unwrap_or_else(|err| panic!("Failed to read {}: {err}", js_path.display()));

    assert!(
        js.contains("snow_sim_bg.wasm"),
        "JS glue does not reference the bundled wasm binary"
    );
    assert!(
        js.contains("export default __wbg_init"),
        "JS glue is missing the default init export"
    );
}
