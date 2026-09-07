# Picalc

A desktop calculator built with [GPUI Kit](https://github.com/longbridge/gpui-kit), ported from omacalc's interaction model: the same keypad, the same percent and chaining semantics, and the same Qt-style number formatting.

The whole calculation engine is pure Rust with no UI imports, so every interaction — keypad clicks, keyboard bindings, chained results, error recovery — is covered by unit tests.

## Install

User-local install (binary, icon, launcher). No root:

```sh
./scripts/install.sh
```

That puts `picalc` on `~/.local/bin` and a desktop entry in the app launcher. Uninstall with `./scripts/uninstall.sh`.

Tagged releases (`v*`) build a Linux x86_64 tarball on GitHub Actions. Unpack it and run `./install.sh` inside.

## Run from source

```sh
cargo run --release
```

## Keyboard

The keypad and the keyboard drive the same engine:

- `0`–`9` type digits; `.` or `,` the decimal point.
- `+` `-` `*` `/` operators; `%` percent; `=` or `Enter` evaluates.
- `Backspace`/`Delete` edit, `C`/`Esc` clears, `S` toggles the sign.
- `Ctrl+C` / `Super+C` copies the result; `Ctrl+V` / `Super+V` pastes a number.
- `Ctrl+Q` quits.

## Behavior notes

- Percent is iOS-style: with a pending `+` or `−`, `x%` means x percent of the running total (`200 + 10 % =` gives `220`); with `×` or `÷` (or standalone) it is `x ÷ 100`.
- Chaining after `=` continues from the exact stored result, not the rounded display, so `1 ÷ 3 = × 3 =` comes back as `1`.
- Numbers show fifteen significant digits, Qt's `QString::number(value, 'g', 15)` style: binary-float noise stays out of the display and larger magnitudes fall back to scientific notation.
- Division by zero shows `Error`; any digit recovers without an explicit clear.
- Sign with nothing typed starts a fresh negative operand, so `4 + ± 2` enters `4 + (−2)`, not `−42`.
- Entries cap at fifteen digits, and editing after `=` picks up the result's digits.

Text follows the desktop text size (`gsettings` `text-scaling-factor`). The default scale of `1.0` is the size the layout is designed around.

Colors come from `~/.local/state/omarchy/current/theme/colors.toml` when present, following system dark/light mode and re-tinting live on a theme switch.

## Fonts

The iA Writer Mono font is bundled under the SIL Open Font License 1.1; see `fonts/OFL.txt`. The font is copyright Information Architects Inc. and based on IBM Plex, copyright IBM Corp.