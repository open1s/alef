# alef-readme

README generator for alef.

alef-readme generates per-language README.md files for binding packages. Each README includes the package name, a description, language-specific installation instructions (pip, npm, gem, composer, mix, go get, Maven, dotnet, CRAN), a quick-start code example, and a link to the main repository documentation. The output paths follow alef's package layout conventions: `packages/{language}/README.md` for most languages, and `crates/{name}-ffi/README.md` or `crates/{name}-wasm/README.md` for FFI and WASM targets.

Metadata such as the package description and repository URL is read from the `[scaffold]` section of alef.toml. All 11 target languages (Python, Node.js, Ruby, PHP, Elixir, Go, Java, C#, FFI, WASM, R) are supported.

Part of the [alef](https://github.com/kreuzberg-dev/alef) polyglot binding generator.

## License

MIT
