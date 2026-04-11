# alef-backend-java

Java (Panama FFM) backend for alef.

This crate generates Java source code that uses the Panama Foreign Function & Memory API (Java 21+) to call into the C FFI layer produced by `alef-backend-ffi`. It produces Java records for data types, enum classes, a raw FFI wrapper class with `Linker.downcallHandle` and `FunctionDescriptor` definitions, and a public facade class with idiomatic Java method signatures. Names that conflict with `java.lang.Object` methods (such as `wait`, `equals`, `hashCode`) are automatically disambiguated.

The backend generates a complete Maven-compatible project structure with package directories derived from the configured Java package name. It depends on `alef-backend-ffi` having generated the C FFI layer first.

Part of the [alef](https://github.com/kreuzberg-dev/alef) polyglot binding generator.

## License

MIT
