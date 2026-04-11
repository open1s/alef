# alef-backend-wasm

WASM (wasm-bindgen) backend for alef.

This crate generates Rust source code that uses [wasm-bindgen](https://rustwasm.github.io/wasm-bindgen/) to expose a Rust library as a WebAssembly module usable from JavaScript and TypeScript. It produces `#[wasm_bindgen]` structs and functions with proper type mappings for the browser and Node.js environments. Async functions are supported through wasm-bindgen-futures.

The backend provides configurable exclusions and overrides via `WasmConfig`: specific functions or types can be excluded from generation, and type mappings can be overridden (for example, remapping `Path` to `String` for WASM compatibility). Copy semantics are tracked for enums and primitives to emit correct clone/move code.

Part of the [alef](https://github.com/kreuzberg-dev/alef) polyglot binding generator.

## License

MIT
