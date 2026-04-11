# alef-backend-php

PHP (ext-php-rs) backend for alef.

This crate generates Rust source code that uses [ext-php-rs](https://github.com/davidcole1340/ext-php-rs) to expose a Rust library as a native PHP extension. It produces `#[php_class]` structs with `#[php_impl]` method blocks, `#[php_function]` free functions, enum constant definitions, and serde-based conversion bridges when available. Async operations are handled via Tokio `block_on` to integrate with PHP's synchronous execution model.

The backend supports DTO generation styles including `readonly-class` and `array`. Code generation is feature-gated, allowing conditional compilation of extension functions based on Cargo features.

Part of the [alef](https://github.com/kreuzberg-dev/alef) polyglot binding generator.

## License

MIT
