# eisberg

Opinionated polyglot binding generator for Rust.

## What it does

eisberg reads a Rust crate's public API surface and generates binding code for up to 10 languages:

- **Python** (PyO3) — binding crate + `.pyi` type stubs
- **Node.js** (NAPI-RS) — binding crate + `.d.ts` (auto)
- **Ruby** (Magnus) — binding crate + `.rbs` type stubs
- **PHP** (ext-php-rs) — binding crate
- **Elixir** (Rustler) — NIF binding crate
- **WebAssembly** (wasm-bindgen) — binding crate with configurable restrictions
- **C** (FFI) — `extern "C"` functions + cbindgen header
- **Go** (cgo) — Go package wrapping C FFI
- **Java** (Panama FFM) — Java package wrapping C FFI
- **C#** (P/Invoke) — .NET package wrapping C FFI

It also generates package scaffolding, READMEs,
and version sync across all language packages.

## Install

```bash
cargo install eisberg-cli
```

or via Homebrew:

```bash
brew install kreuzberg-dev/tap/eisberg
```

## Quick Start

1. Create a `eisberg.toml` in your Rust workspace root:

```toml
[crate]
name = "my-lib"
sources = ["crates/my-lib/src/lib.rs"]
version_from = "Cargo.toml"

languages = ["python", "node", "ffi", "go"]

[python]
module_name = "_my_lib"

[ffi]
prefix = "my_lib"

[go]
module = "github.com/my-org/my-lib-go"
```

2. Generate bindings:

```bash
eisberg generate
```

3. Verify bindings are up to date (CI):

```bash
eisberg verify --exit-code
```

## Commands

Show the full list with `eisberg --help`. Key commands:

| Command | Description |
|---------|-------------|
| `eisberg generate` | Generate binding code for all/selected languages |
| `eisberg generate --lang python,node` | Generate specific languages only |
| `eisberg generate --clean` | Regenerate ignoring cache |
| `eisberg stubs` | Generate type stubs (.pyi, .rbs) |
| `eisberg scaffold` | Generate package metadata (pyproject.toml, etc.) |
| `eisberg readme` | Generate per-language README files |
| `eisberg sync-versions` | Sync Cargo.toml version to all manifests |
| `eisberg verify --exit-code` | CI gate: fail if any binding is stale |
| `eisberg diff` | Show what would change without writing |
| `eisberg lint` | Run configured linters on generated output |
| `eisberg all` | Run everything |
| `eisberg init` | Create a eisberg.toml interactively |
| `eisberg cache clear` | Clear the build cache |

## How It Works

```text
eisberg.toml + Rust pub API
        |
        v
   eisberg extract          (syn parses pub types/fns/enums)
        |
        v
   IR (ApiSurface)       (serializable JSON, cached in .eisberg/)
        |
        v
   eisberg generate         (per-language backends emit code)
        |
        +-> crates/{name}-py/src/     (PyO3 binding crate)
        +-> crates/{name}-node/src/   (NAPI-RS binding crate)
        +-> crates/{name}-ffi/src/    (C FFI layer)
        +-> packages/go/              (cgo wrapper)
        +-> ...
```

## Caching

eisberg caches the extracted IR and per-language output hashes
in `.eisberg/` (gitignored). Only backends whose inputs changed
are regenerated. Use `--clean` to bypass.

## Verification

`eisberg verify` checks:

1. **Staleness** — are generated files up to date with Rust source?
2. **API parity** — does every language expose the same types/functions?
3. **Type stub consistency** — do .pyi/.rbs match generated bindings?

Use in CI:

```yaml
- run: eisberg verify --exit-code
```

Or as a pre-commit hook via prek.

## Configuration Reference

See `eisberg.reference.toml` for the full documented config schema.

## License

MIT
