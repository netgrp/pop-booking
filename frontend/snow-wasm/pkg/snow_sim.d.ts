/* tslint:disable */
/* eslint-disable */
export function wasm_start(): void;
export function snow_init(width: number, height: number): void;
export function snow_resize(width: number, height: number): void;
export function snow_pointer_wind(target: number): void;
export function snow_pointer_move(x: number, y: number, dx: number, dy: number): void;
export function snow_step(delta_ms: number): void;
export function snow_reset(): void;
export function snow_set_tint(r: number, g: number, b: number): void;
export function snow_dynamic_count(): number;
export function snow_inactive_count(): number;
export function snow_pile_bins(): Float32Array;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly wasm_start: () => void;
  readonly snow_init: (a: number, b: number) => [number, number];
  readonly snow_resize: (a: number, b: number) => [number, number];
  readonly snow_pointer_wind: (a: number) => void;
  readonly snow_pointer_move: (a: number, b: number, c: number, d: number) => [number, number];
  readonly snow_step: (a: number) => [number, number];
  readonly snow_reset: () => [number, number];
  readonly snow_set_tint: (a: number, b: number, c: number) => [number, number];
  readonly snow_dynamic_count: () => number;
  readonly snow_inactive_count: () => number;
  readonly snow_pile_bins: () => [number, number, number];
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly __wbindgen_free: (a: number, b: number, c: number) => void;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __externref_table_dealloc: (a: number) => void;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
