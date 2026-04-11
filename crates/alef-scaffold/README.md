# alef-scaffold

Package scaffolding generator for alef.

alef-scaffold generates complete package manifest and build configuration files for each target language. For languages with native Rust binding crates (Python, Node.js, Ruby, PHP, Elixir, R), it generates both the language-side manifest and the Rust binding crate's Cargo.toml. Supported outputs include: pyproject.toml (Python/maturin), package.json (Node.js/NAPI-RS), .gemspec (Ruby/Magnus), composer.json (PHP/ext-php-rs), mix.exs (Elixir/Rustler), go.mod (Go/cgo), pom.xml (Java/Panama FFM), .csproj (C#/P/Invoke), DESCRIPTION (R/extendr), and Cargo.toml files for Rust binding crates and FFI/WASM targets.

Scaffold metadata (description, license, repository, authors, keywords) is read from the `[scaffold]` section of alef.toml. Per-language feature overrides are propagated to the generated Cargo.toml dependency declarations.

Part of the [alef](https://github.com/kreuzberg-dev/alef) polyglot binding generator.

## License

MIT
