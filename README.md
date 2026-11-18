# en_get

One-click environment setup for students: copies bundled **Pi**, **R** and
**Typst** config files into the right folders. It's a single `.exe` with no
dependencies, so students just double-click it.

## For students
1. Download `en_get.exe` and double-click it.
2. If Windows SmartScreen appears, click **More info → Run anyway**.
3. Read the summary and press Enter. Existing files that differ are kept as `*.bak`.

Options (from a terminal): `en_get --help`, `--only pi|r|typst`, `--dry-run`.

## For maintainers
Put the config files into `assets/pi`, `assets/r` and `assets/typst` (see
`assets/README.md`), then:

```sh
cargo test
cargo run -- --root /tmp/fake-home --dry-run --no-pause   # preview on the Mac
cargo build --release --target x86_64-pc-windows-gnu
# -> target/x86_64-pc-windows-gnu/release/en_get.exe
```

Cross-compiling from macOS needs `rustup target add x86_64-pc-windows-gnu` and
`brew install mingw-w64`. The linker is set in `.cargo/config.toml`.
