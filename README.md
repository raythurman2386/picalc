# Picalc

A calculator for the [pi suite](https://github.com/raythurman2386), built with
[GPUI Kit](https://github.com/longbridge/gpui-kit) — a small, native,
theme-following desktop calculator written for Raspberry Pi 5-class hardware
(and happy on any Linux desktop). Ported from omacalc's interaction model: the
same keypad, the same percent and chaining semantics, and the same Qt-style
number formatting.

## Features

- **Keypad and keyboard drive the same engine** — click or type; chained
  results, editing, and error recovery behave identically either way.
- **iOS-style percent**: with a pending `+` or `−`, `x%` means x percent of
  the running total (`200 + 10 % =` gives `220`); with `×` or `÷` (or
  standalone) it is `x ÷ 100`.
- **Exact chaining**: after `=`, the next operation continues from the exact
  stored result, not the rounded display, so `1 ÷ 3 = × 3 =` comes back as
  `1`.
- **Qt-style formatting**: fifteen significant digits
  (`QString::number(value, 'g', 15)`) keep binary-float noise out of the
  display; larger magnitudes fall back to scientific notation. Entries cap at
  fifteen digits.
- **Forgiving errors**: division by zero shows `Error`, and the next digit
  recovers without an explicit clear. Sign with nothing typed starts a fresh
  negative operand, so `4 + ± 2` enters `4 + (−2)`, not `−42`.
- **Aesthetic**: keyboard-first, follows the desktop dark/light mode and text
  scale, and live re-tints from the Omarchy theme palette.

The calculation engine is pure Rust with no UI imports, so every interaction —
keypad clicks, keyboard bindings, chained results, error recovery — is covered
by unit tests (30 across the suite).

## Install

User-local install from a tagged release (no root, Ed25519-verified,
fail-closed):

```sh
curl -fsSL https://raw.githubusercontent.com/raythurman2386/picalc/main/scripts/netinstall.sh | bash
```

Or build and install from source:

```sh
cargo build --release
./scripts/install.sh
```

Uninstall with `./scripts/uninstall.sh`. The netinstaller accepts a `--prefix`
directory, an optional version argument, and `--force`; the source install
honors `PREFIX=DIR`.

Tagged `v*` releases also build x86_64 + aarch64 tarballs on GitHub Actions
(glibc 2.39+ — e.g. Raspberry Pi OS / Debian 13). Unpack the one for your
architecture and run `./install.sh` inside.

Releases are authenticated with Ed25519 signatures over `checksums.txt`; the
public key is committed as `picalc-signing-key.pub` and pinned in the
installer, which refuses anything it cannot verify.

## Keyboard

| Keys | Action |
|---|---|
| `0`–`9` | Digits; `.` or `,` the decimal point |
| `+` `-` `*` `/` | Operators; `%` percent |
| `=` / `enter` | Evaluate |
| `Backspace` / `Delete` | Edit the entry |
| `c` / `Esc` | Clear · `s` toggle sign |
| `Ctrl+C` / `Super+C` | Copy the result |
| `Ctrl+V` / `Super+V` | Paste a number |
| `Ctrl+Q` | Quit |

## State and theming

Colors follow the desktop theme —
`~/.local/state/omarchy/current/theme/colors.toml` when present — re-tinting
live on theme switches; text follows the desktop text scale
(`gsettings` `text-scaling-factor`). The default scale of `1.0` is the size
the layout is designed around.

## Fonts

The iA Writer Mono font is bundled under the SIL Open Font License 1.1; see
`fonts/OFL.txt`. The font is copyright Information Architects Inc. and based
on IBM Plex, copyright IBM Corp.

## Development

```sh
cargo fmt --check          # formatting
cargo clippy --all-targets -- -D warnings
cargo test                 # 30 tests
cargo run --release        # calculate
```

CI runs fmt, clippy, and tests on every push; tagged `v*` releases build
x86_64 + aarch64 tarballs (glibc 2.39+) with an install smoke test, and the
netinstall integrity harness can be run locally with
`bash scripts/test-netinstall.sh`.

## License

MIT — see [LICENSE](LICENSE). Bundled fonts: SIL OFL 1.1 (see above).