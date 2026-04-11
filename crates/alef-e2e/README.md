# alef-e2e

Fixture-driven e2e test generator for alef.

alef-e2e reads JSON fixture files and generates complete, self-contained end-to-end test projects for each supported language. Fixtures are loaded from a configurable directory, validated, and grouped by category. Each language implements the `E2eCodegen` trait, which produces test files, build manifests, and assertion helpers tailored to that language's test framework. The generator handles language-specific string escaping, field access patterns, and test scaffold generation.

Supported target languages are: Rust (cargo test), Python (pytest), TypeScript (vitest), Ruby (RSpec), PHP (PHPUnit), Go (go test), Java (JUnit), C# (xUnit), Elixir (ExUnit), R (testthat), WASM (vitest), and C (CMake/CTest). Generated projects are written to `e2e/{language}/` directories and include all necessary configuration to run immediately with the language's standard test runner.

Part of the [alef](https://github.com/kreuzberg-dev/alef) polyglot binding generator.

## License

MIT
