# alef-backend-rustler

Elixir (Rustler) backend for alef.

This crate generates Rust source code that uses [Rustler](https://github.com/rusterlium/rustler) to expose a Rust library as an Elixir NIF (Native Implemented Function). It produces `#[rustler::nif]` functions, `#[derive(NifStruct)]` and `#[derive(NifUnitEnum)]` types for automatic Elixir term encoding/decoding, and `ResourceArc`-wrapped opaque types for sharing Rust state across NIF calls. Module prefixes are derived from the Elixir module namespace configuration.

The backend supports DTO generation styles including `struct` and `typed-struct`. Opaque types are wrapped in `Arc` and registered as Rustler resources with proper lifecycle management.

Part of the [alef](https://github.com/kreuzberg-dev/alef) polyglot binding generator.

## License

MIT
