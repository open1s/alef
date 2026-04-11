# alef-backend-csharp

C# (P/Invoke) backend for alef.

This crate generates C# source code that uses P/Invoke (`[DllImport]`) to call into the C FFI layer produced by `alef-backend-ffi`. It produces a `NativeMethods` class with `extern` declarations for all FFI functions, C# record types for data structures, enum definitions, typed exception classes derived from `thiserror` error enums, and a public wrapper class with idiomatic C# method signatures and namespace organization.

The backend generates files organized by the configured C# namespace, with each type in its own file following .NET conventions. It depends on `alef-backend-ffi` having generated the C FFI layer first.

Part of the [alef](https://github.com/kreuzberg-dev/alef) polyglot binding generator.

## License

MIT
