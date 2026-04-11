# alef-backend-napi

Node.js (NAPI-RS) backend for alef.

This crate generates Rust source code that uses the [NAPI-RS](https://napi.rs/) framework to expose a Rust library as a native Node.js addon. It produces `#[napi]` structs and functions, `#[napi(constructor)]` constructors, `#[napi(string_enum)]` enums, and native async functions that return JavaScript Promises. Generated struct names are prefixed with `Js` to avoid collisions with core Rust types.

The backend supports DTO generation styles including `interface` and `zod`. It includes a post-build step that patches generated `.d.ts` TypeScript declaration files to ensure correct type definitions for the published npm package.

Part of the [alef](https://github.com/kreuzberg-dev/alef) polyglot binding generator.

## License

MIT
