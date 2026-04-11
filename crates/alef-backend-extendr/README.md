# alef-backend-extendr

R (extendr) backend for alef.

This crate generates Rust source code that uses [extendr](https://extendr.github.io/extendr/) to expose a Rust library as an R package native extension. It produces `#[extendr]`-annotated functions and type wrappers with R-compatible type mappings, where integer types narrower than 64 bits map to `i32` and all 64-bit and floating-point types map to `f64` to match R's numeric representation. JSON values are passed as strings with serde serialization.

The backend supports DTO generation styles including `list` and `r6`. Async operations are not supported due to R's single-threaded execution model.

Part of the [alef](https://github.com/kreuzberg-dev/alef) polyglot binding generator.

## License

MIT
