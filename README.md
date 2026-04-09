<div align="center">

<img width="100%" alt="kreuzberg.dev banner" src="https://github.com/user-attachments/assets/1b6c6ad7-3b6d-4171-b1c9-f2026cc9deb8" />

<div style="display: flex; gap: 8px; justify-content: center; flex-wrap: wrap; margin-top: 16px;">

<a href="https://crates.io/crates/alef-cli">
  <img src="https://img.shields.io/crates/v/alef-cli?label=crates.io&color=007ec6" alt="crates.io">
</a>
<a href="https://github.com/kreuzberg-dev/alef/releases">
  <img src="https://img.shields.io/github/v/release/kreuzberg-dev/alef?label=Release&color=007ec6" alt="Release">
</a>
<a href="https://github.com/kreuzberg-dev/alef/actions/workflows/ci.yml">
  <img src="https://img.shields.io/github/actions/workflow/status/kreuzberg-dev/alef/ci.yml?label=CI&color=007ec6" alt="CI">
</a>
<a href="https://github.com/kreuzberg-dev/alef/blob/main/LICENSE">
  <img src="https://img.shields.io/badge/License-MIT-007ec6" alt="License">
</a>

</div>

<br>

<a href="https://discord.gg/xt9WY3GnKR">
  <img height="22" src="https://img.shields.io/badge/Discord-Join%20our%20community-7289da?logo=discord&logoColor=white" alt="Discord">
</a>

</div>

# Alef

**Opinionated polyglot binding generator for Rust libraries.**

Alef generates production-quality, fully-typed, lint-clean bindings for **11 languages** from a single Rust crate and a TOML config file. One command to generate. One command to build. One command to test.

## Supported Languages

| Language | Framework | Package Format | Test Framework |
|----------|-----------|----------------|----------------|
| Python | PyO3 | PyPI (.whl) | pytest |
| TypeScript/Node.js | NAPI-RS | npm | vitest |
| WebAssembly | wasm-bindgen | npm | vitest |
| Ruby | Magnus | RubyGems (.gem) | RSpec |
| PHP | ext-php-rs | Composer | PHPUnit |
| Go | cgo + FFI | Go modules | go test |
| Java | Panama FFM | Maven (.jar) | JUnit |
| C# | P/Invoke | NuGet (.nupkg) | xUnit |
| Elixir | Rustler NIF | Hex | ExUnit |
| R | extendr | CRAN | testthat |
| C | cbindgen | Header (.h) | -- |

## Quick Start

### Install

```bash
# From crates.io
cargo install alef-cli

# Or via Homebrew
brew install kreuzberg-dev/tap/alef

# Or from source
git clone https://github.com/kreuzberg-dev/alef.git
cd alef && cargo install --path crates/alef-cli
```

### Initialize

```bash
cd your-rust-crate
alef init --lang python,node,ruby,go
```

This creates `alef.toml` with your crate's configuration.

### Generate Bindings

```bash
alef generate              # Generate all configured languages
alef generate --lang node  # Generate for specific language
alef generate --clean      # Regenerate everything (ignore cache)
```

### Build

```bash
alef build                 # Build all languages
alef build --lang node     # Build Node.js (runs napi build + patches .d.ts)
alef build --release       # Release profile
```

### Test

```bash
alef test                  # Run all language tests
alef test --e2e            # Include e2e tests
alef test --lang python,go # Specific languages
```

## Commands

| Command | Description |
|---------|-------------|
| `alef init` | Initialize `alef.toml` for your crate |
| `alef extract` | Extract API surface from Rust source into IR JSON |
| `alef generate` | Generate language bindings from IR |
| `alef stubs` | Generate type stubs (.pyi, .rbs, .d.ts) |
| `alef scaffold` | Generate package manifests (pyproject.toml, package.json, etc.) |
| `alef readme` | Generate per-language README files |
| `alef build` | Build bindings with native tools (maturin, napi, wasm-pack, etc.) |
| `alef test` | Run per-language test suites |
| `alef lint` | Run configured linters on generated output |
| `alef sync-versions` | Sync version from Cargo.toml to all manifests |
| `alef verify` | Check if bindings are up-to-date |
| `alef diff` | Show what would change without writing |
| `alef all` | Run full pipeline: generate + stubs + scaffold + readme + sync |
| `alef cache` | Manage build cache |

## Configuration

Alef is configured via `alef.toml` in your project root:

```toml
[crate]
name = "my-library"
sources = [
  "crates/my-library/src/lib.rs",
  "crates/my-library/src/types.rs",
]

languages = ["python", "node", "ruby", "go", "java", "csharp", "elixir", "wasm", "ffi"]

[output]
python = "crates/my-library-py/src/"
node = "crates/my-library-node/src/"
ffi = "crates/my-library-ffi/src/"

[python]
module_name = "my_library"

[node]
package_name = "@myorg/my-library"

[ffi]
prefix = "ml"
header_name = "my_library.h"

[dto]
python = "dataclass"
python_output = "typeddict"
node = "interface"

[test.python]
command = "pytest packages/python/tests/"
e2e = "cd e2e/python && pytest"

[test.node]
command = "npx vitest run"
```

## Features

### Configurable DTO Styles

Choose how types are represented in each language:

| Language | Options |
|----------|---------|
| Python | `dataclass`, `typeddict`, `pydantic`, `msgspec` |
| TypeScript | `interface`, `zod` |
| Ruby | `struct`, `dry-struct`, `data` |
| PHP | `readonly-class`, `array` |

### Visitor FFI

Full 40-method visitor callback support via C FFI, enabling visitor patterns in Go, Java, C#, and other FFI-based languages.

### Version Sync

```bash
alef sync-versions              # Sync from Cargo.toml
alef sync-versions --bump patch # Bump + sync + refresh headers
```

Supports text replacements for C headers, pkg-config files, READMEs, and custom patterns via `[sync]` config.

### Build Orchestration

Wraps language-specific build tools with post-processing:

| Language | Tool | Post-Processing |
|----------|------|-----------------|
| Python | maturin | -- |
| Node.js | napi-rs | Patches `.d.ts` (const enum to enum for verbatimModuleSyntax) |
| WASM | wasm-pack | -- |
| Go/Java/C# | cargo (FFI) | Generates C header via cbindgen |

## Architecture

```text
alef.toml --> Extract IR --> Generate Bindings --> Build --> Test
                 |                  |
            API Surface       11 Language Backends
            (types, fns,      (PyO3, NAPI, Magnus,
             enums, errors)    ext-php-rs, wasm-bindgen,
                               cbindgen, Rustler, extendr,
                               Panama FFM, P/Invoke, cgo)
```

**Crate structure:**

- `alef-core` -- IR types, config schema, Backend trait
- `alef-extract` -- Rust source to IR extraction via `syn`
- `alef-codegen` -- Shared code generation utilities
- `alef-backend-*` -- 11 language-specific backends
- `alef-cli` -- CLI binary with all commands

## Contributing

Contributions welcome! Please open an issue or PR on [GitHub](https://github.com/kreuzberg-dev/alef).

## License

[MIT](LICENSE) -- Copyright (c) 2025-2026 Kreuzberg, Inc.
