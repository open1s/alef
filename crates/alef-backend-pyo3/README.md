# alef-backend-pyo3

Python (PyO3) backend for alef.

This crate generates Rust source code that uses the [PyO3](https://pyo3.rs/) framework to expose a Rust library as a native Python extension module. It produces `#[pyclass]` structs with `#[pyo3(get)]` field accessors, `#[pymethods]` blocks with `#[new]` constructors and `#[staticmethod]` annotations, `#[pyfunction]` free functions, and `From` trait implementations for type conversions. Async functions are wrapped using `pyo3_async_runtimes` to return Python awaitables.

The backend supports multiple DTO generation styles selectable via configuration: `dataclass`, `typeddict`, `pydantic`, and `msgspec`. It also generates `.pyi` type stub files for full IDE autocompletion and static type checking support. When serde is available in the output crate, serde-based parameter conversion bridges are emitted automatically.

Part of the [alef](https://github.com/kreuzberg-dev/alef) polyglot binding generator.

## License

MIT
