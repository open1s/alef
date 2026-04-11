# alef-cli

CLI for the alef polyglot binding generator.

alef-cli is the binary crate that provides the `alef` command-line tool. It orchestrates the full binding generation pipeline by wiring together extraction, code generation, scaffolding, and testing. Commands include: `init` (create alef.toml), `extract` (parse Rust source into IR), `generate` (produce binding code), `stubs` (generate type stubs like .pyi and .d.ts), `scaffold` (generate package manifests), `readme` (generate per-language READMEs), `build` (compile bindings with native tools like maturin, napi, wasm-pack), `test` (run language test suites), `lint` (run language linters), `sync-versions` (propagate version from Cargo.toml), `verify` (check bindings are up to date), `diff` (preview changes), `e2e` (generate and run e2e test suites from fixtures), `cache` (manage build cache), and `all` (run the complete pipeline).

The CLI uses clap for argument parsing, supports a `--config` flag for custom alef.toml paths, and includes a backend registry that wires all 11 language backends (Python, TypeScript, Ruby, PHP, Go, Java, C#, Elixir, R, WASM, FFI/C).

Part of the [alef](https://github.com/kreuzberg-dev/alef) polyglot binding generator.

## License

MIT
