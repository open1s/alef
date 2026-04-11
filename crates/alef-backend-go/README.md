# alef-backend-go

Go (cgo) backend for alef.

This crate generates pure Go source code that wraps the C FFI layer produced by `alef-backend-ffi`. It produces Go structs, methods, and free functions that call through cgo to the underlying Rust library, handling C string conversions, pointer lifecycle management with `defer` cleanup, serde rename strategies for struct field JSON tags, and Go-idiomatic error returns. The generated Go package includes `#cgo` directives for linking against the compiled FFI shared library.

The backend derives the Go package name from the configured module path and supports configurable output directories. It depends on `alef-backend-ffi` having generated the C FFI layer and header files first.

Part of the [alef](https://github.com/kreuzberg-dev/alef) polyglot binding generator.

## License

MIT
