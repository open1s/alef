# alef-adapters

FFI adapter pattern generators for alef.

alef-adapters generates adapter code that bridges language-specific calling conventions with the Rust core library. It supports several adapter patterns: `SyncFunction` for synchronous free function calls, `AsyncMethod` for async method wrappers, `CallbackBridge` for callback-based interop (generating both bridge struct and impl blocks), `Streaming` for iterator/stream wrappers (generating method bodies and optional iterator struct definitions), and `ServerLifecycle` for server start/stop patterns.

The crate reads adapter definitions from the `[[adapters]]` section of alef.toml and produces per-language method/function bodies keyed by `"TypeName.method_name"` or `"function_name"`, which backends then splice into their generated binding code.

Part of the [alef](https://github.com/kreuzberg-dev/alef) polyglot binding generator.

## License

MIT
