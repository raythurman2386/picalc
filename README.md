# Picalc

A desktop calculator built with [GPUI Kit](https://github.com/longbridge/gpui-kit), ported from omacalc's interaction model: the same keypad, the same percent and chaining semantics, and the same Qt-style number formatting.

The whole calculation engine is pure Rust with no UI imports, so every interaction — keypad clicks, keyboard bindings, chained results, error recovery — is covered by unit tests.

## Install

User-local install (binary, icon, launcher). No root:

```sh
./scripts/install.sh
```

That puts `picalc` on `~/.local/bin` and a desktop entry in the app launcher. Uninstall with `./scripts/uninstall.sh`.

Tagged releases (`v*`) build Linux tarballs on GitHub Actions for x86_64 and aarch64 (Raspberry Pi 5 and other 64-bit ARM boards), each requiring glibc 2.39+ (Debian 13, Ubuntu 24.04, current Raspberry Pi OS). Unpack the one for your machine and run `./install.sh` inside.

## Release signing

Every release's `checksums.txt` is signed with an Ed25519 key, so an installer can prove the checksums (and therefore the tarball) came from this repo:

- `bash scripts/gen-signing-key.sh` generates the keypair into `~/.picalc/signing` — the secret key stays offline forever and is never committed, used in CI, or uploaded. Only the public key is committed (`picalc-signing-key.pub`) and pinned in the installers.
- `bash scripts/sign-release.sh CHECKSUMS_FILE SECRET_KEY` signs one file; `scripts/sign-releases.sh VERSION...` batch-signs published releases offline into `~/.picalc/signing/releases/<version>/`; `scripts/upload-release-sigs.sh VERSION...` attaches each `checksums.txt.sig` back to its release with `gh release upload --clobber`.

Verification on the installer side is fail-closed: a release without a signature, or whose signature does not verify against the pinned public key, is refused.

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