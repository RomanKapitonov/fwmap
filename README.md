# fwmap

Firmware memory reporting and budget validation for Rust embedded projects.

Builds your firmware via `cargo build` and emits a JSON report: ELF section sizes,
linker overflow diagnostics, and the top RAM/BSS symbols by size. Reports can be
validated against a JSON budget file for use as a CI gate.

The key capability over `cargo-size`: works even when the firmware **fails to link**
due to memory overflow. It falls back to the linker `.map` file for section sizes and
parses linker error output for overflow amounts, letting you see exactly what is over
budget and by how much.

## Integration (git subtree)

Add to your project:

```sh
git subtree add --prefix fwmap <repo-url> main --squash
```

Add `"fwmap"` to your workspace `Cargo.toml` members:

```toml
[workspace]
members = ["fwmap", ...]
```

Add a cargo alias in `.cargo/config.toml`:

```toml
[alias]
xtask = "run -p fwmap --"
```

Set project-specific defaults by editing the `default_value` attributes in
`fwmap/src/cli.rs` — at minimum `--package` and `--target`.

Update later:

```sh
git subtree pull --prefix fwmap <repo-url> main --squash
```

## Commands

### `cargo xtask firmware-report`

Build firmware and emit a JSON memory report to stdout (or `--output <path>`).

```
Options:
  --package <PKG>       Cargo package to build [default: stm32f429zi-example]
  --output <PATH>       Output path for JSON (stdout if omitted)
  --target <TRIPLE>     Cargo target triple [default: thumbv7em-none-eabihf]
  --profile <PROFILE>   Cargo profile [default: release]
  --features <F,...>    Feature flags, comma-separated, repeatable
  --allow-build-failure Emit report even when the build fails
```

**Default features:** When `--features` is not specified, the defaults in `fwmap/src/cli.rs`
are applied (currently `effect-runtime-experimental-dag,ccmram`). Update these defaults
when integrating into a new project.

Use `--allow-build-failure` when the firmware may not fit yet and you still want
section sizes and overflow amounts.

### `cargo xtask firmware-validate`

Build firmware and validate memory usage against a JSON budget. Exits non-zero if
any constraint is violated.

```
Options:
  --package <PKG>   [default: stm32f429zi-example]
  --budget <PATH>   [default: support/memory/firmware-memory-budget.json]
  --target <TRIPLE> [default: thumbv7em-none-eabihf]
  --profile <PROFILE> [default: release]
  --features <F,...>
```

## Budget File Format

`firmware-memory-budget.json`:

```json
{
  "require_successful_build": true,
  "require_valid_stack_placement": true,
  "max_ram_overflow_bytes": 0,
  "max_uninit_overflow_bytes": 0,
  "max_section_bytes": {
    ".ccmram": 65536,
    ".bss": 131072
  },
  "max_symbol_prefix_bytes": {
    "my_crate::module::LARGE_BUFFER": 65536
  }
}
```

| Field | Description |
|-------|-------------|
| `require_successful_build` | Fail if the build exits non-zero |
| `require_valid_stack_placement` | Fail if the linker reports invalid stack placement |
| `max_ram_overflow_bytes` | Max allowed RAM overflow in bytes (0 = must fit) |
| `max_uninit_overflow_bytes` | Max allowed `.uninit` overflow in bytes |
| `max_section_bytes` | Per-section size limits. Tracked sections: `.text`, `.rodata`, `.data`, `.bss`, `.uninit`, `.ccmram`, `.sdram` |
| `max_symbol_prefix_bytes` | Size limit for the largest symbol whose name starts with the given prefix (matched against the top-N symbols from the map file). Validation fails if the prefix is not found — only use for symbols guaranteed to exist in the binary. |

## Report Format

`cargo xtask firmware-report` emits JSON:

```json
{
  "schema_version": 1,
  "build": {
    "target": "thumbv7em-none-eabihf",
    "profile": "release",
    "features": ["ccmram"],
    "succeeded": true,
    "exit_code": 0,
    "map_path": "target/firmware.map",
    "elf_path": "target/thumbv7em-none-eabihf/release/my-firmware"
  },
  "linker": {
    "ram_overflow_bytes": 0,
    "uninit_overflow_bytes": 0,
    "stack_placement_failed": false
  },
  "sections": {
    ".bss": 12345,
    ".ccmram": 4096,
    ".text": 98304
  },
  "top_symbols": [
    {
      "section": ".bss",
      "size_bytes": 8192,
      "address": "20000000",
      "symbol": "my_crate::module::POOL"
    }
  ]
}
```

When the build fails before producing an ELF, `elf_path` is `null` and sections are
read from the `.map` file instead.
