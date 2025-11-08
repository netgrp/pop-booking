# Snow WASM Module

This crate now owns the full WebGL-powered snow renderer. Rust (compiled to WebAssembly via `wasm-bindgen`) bootstraps the WebGL2 context, runs the GPU simulation shaders, and renders directly to `#snow-canvas`. The browser-side script simply drives pointer wind, resize events, and the frame loop.

## Building

The build pipeline relies on `wasm-pack` so that the generated JS glue (`snow_sim.js`) is available for dynamic `import()` in `frontend/script.js`.

```bash
cd frontend/snow-wasm
./build.sh
```

> **Prerequisite:** install the matching `wasm-bindgen` CLI (`cargo install wasm-bindgen-cli --version 0.2.104`) and ensure the `wasm32-unknown-unknown` target is added via `rustup target add wasm32-unknown-unknown`.

The build wraps two steps:

1. `cargo build --release --target wasm32-unknown-unknown`
2. `wasm-bindgen --target web --out-dir pkg --out-name snow_sim target/wasm32-unknown-unknown/release/snow_wasm.wasm`

The resulting JS/WASM bundle lives in `frontend/snow-wasm/pkg/`, which the frontend dynamically imports (`./snow-wasm/pkg/snow_sim.js`).

## Development tips

- All shader source lives in `src/shaders.rs`. Updating the GLSL and rebuilding is enough; no JS changes are required.
- `snow_init`, `snow_resize`, `snow_pointer_wind`, `snow_step`, `snow_set_tint`, etc. are exported via `wasm-bindgen` and are called from `frontend/script.js`.
- The renderer maintains its own ping-pong textures; there is no longer a CPU particle buffer, so tools that previously inspected memory should now look at WebGL debugging overlays.
