# fwmap

Firmware memory reporting and budget validation for Rust embedded projects.

Builds your firmware via `cargo build` and emits a JSON report: ELF section
sizes, linker overflow diagnostics, and the top RAM/BSS symbols by size.
Reports can be validated against a JSON budget file for use as a CI gate.

The key capability over `cargo-size`: works even when the firmware **fails
to link** due to memory overflow. It falls back to the linker `.map` file
for section sizes and parses linker error output for overflow amounts,
letting you see exactly what is over budget and by how much.

## Integration (git submodule)

```sh
git submodule add https://github.com/RomanKapitonov/fwmap fwmap
```

Exclude fwmap from your workspace (in root `Cargo.toml`):

```toml
[workspace]
exclude = ["fwmap"]
```

Add a cargo alias in `.cargo/config.toml`:

```toml
[alias]
fwmap = "run --locked --manifest-path fwmap/Cargo.toml --"
```

Create `fwmap.toml` at your repo root with your project defaults:

```toml
package = "my-firmware"
target = "thumbv7em-none-eabihf"
features = ["my-feature"]
no-default-features = true
budget = "support/memory/firmware-memory-budget.json"
top-symbols = 20
```

Update later:

```sh
git submodule update --remote fwmap
```

## fwmap.toml reference

All fields optional. Use `--<flag>` to override per-invocation.

| Key | Type | Description |
|---|---|---|
| `package` | string | Cargo package to build |
| `target` | string | Cargo target triple |
| `profile` | string | Cargo profile (default: `release`) |
| `features` | array<string> | Cargo features to enable |
| `no-default-features` | bool | Disable the firmware crate's default features |
| `budget` | path | Path to budget JSON (used by `firmware-validate`) |
| `top-symbols` | int | Number of top symbols in report (default: 20) |
| `map-path` | path | Override linker .map location (default: `<target_dir>/firmware.map`) |

fwmap discovers `fwmap.toml` by walking up from the current directory,
same algorithm as `rustfmt.toml` and `.editorconfig`.

## Commands

### `cargo fwmap`

Build firmware and emit a JSON memory report to stdout. Equivalent to
`cargo fwmap firmware-report --allow-build-failure`.

### `cargo fwmap firmware-report`

Build firmware and emit a JSON memory report to stdout (or `--output <path>`).

```
Options:
  --package <PKG>          Override fwmap.toml `package`
  --output <PATH>          Output path for JSON (stdout if omitted)
  --target <TRIPLE>        Override fwmap.toml `target`
  --profile <PROFILE>      Override fwmap.toml `profile`
  --features <F,...>       Override fwmap.toml `features` (comma-separated, repeatable)
  --no-default-features    Disable the firmware crate's default features
  --map-path <PATH>        Override the linker .map path
  --allow-build-failure    Emit report even when the build fails
  --top-symbols <N>        Override fwmap.toml `top-symbols`
```

Use `--allow-build-failure` when the firmware may not fit yet and you
still want section sizes and overflow amounts.

### `cargo fwmap firmware-validate`

Build firmware and validate memory usage against a JSON budget. Exits
non-zero if any constraint is violated.

```
Options:
  --package <PKG>
  --budget <PATH>
  --target <TRIPLE>
  --profile <PROFILE>
  --features <F,...>
  --no-default-features
  --map-path <PATH>
```

## Budget file format

`firmware-memory-budget.json`:

```json
{
  "require_successful_build": true,
  "require_valid_stack_placement": true,
  "max_ram_overflow_bytes": 0,
  "max_uninit_overflow_bytes": 0,
  "max_section_bytes": {
    "Ccmram": 65536,
    "Bss": 131072
  },
  "max_symbol_prefix_bytes": {
    "optional_symbol_prefix": 1024
  },
  "required_symbols": {
    "my_crate::module::LARGE_BUFFER": 65536
  }
}
```

| Field | Description |
|---|---|
| `require_successful_build` | Fail if the build exits non-zero (default: `false`). |
| `require_valid_stack_placement` | Fail if the linker reports invalid stack placement (default: `false`). |
| `max_ram_overflow_bytes` | Max allowed RAM overflow in bytes (0 = must fit). |
| `max_uninit_overflow_bytes` | Max allowed `.uninit` overflow in bytes. |
| `max_section_bytes` | Per-section size limits. Keys are `Section` variant identifiers (`Text`, `Rodata`, `Data`, `Bss`, `Uninit`, `Ccmram`, `Sdram`). |
| `max_symbol_prefix_bytes` | Size limit for the largest symbol whose name starts with the given prefix. If no top-N symbol matches the prefix, the constraint is silently skipped (the symbol may have been inlined or DCE-stripped). Use for size-only budgets on symbols whose presence is not guaranteed. |
| `required_symbols` | Like `max_symbol_prefix_bytes` but fails validation if no top-N symbol matches the prefix. Use when both "must exist" and "must fit" are required. |

## Report format

`cargo fwmap firmware-report` emits JSON:

```json
{
  "build": {
    "target": "thumbv7em-none-eabihf",
    "profile": "release",
    "features": ["my-feature"],
    "succeeded": true,
    "exit_code": 0,
    "map_path": "target/firmware.map",
    "elf_path": "target/thumbv7em-none-eabihf/release/my-firmware",
    "section_source": "elf"
  },
  "linker": {
    "ram_overflow_bytes": 0,
    "uninit_overflow_bytes": 0,
    "stack_placement_failed": false
  },
  "sections": {
    "Bss": 12345,
    "Ccmram": 4096,
    "Text": 98304
  },
  "top_symbols": [
    {
      "section": "Bss",
      "size_bytes": 8192,
      "address": 536870912,
      "symbol": "my_crate::module::POOL"
    }
  ]
}
```

The `build.section_source` field records whether section byte counts came
from the ELF (`"elf"`), the linker map (`"map"`), or neither (`"none"`,
when the build failed before producing either file). When the build fails
before producing an ELF, `elf_path` is `null` and sections are read from
the `.map` file instead. Symbol addresses are emitted as numeric `u64`.
