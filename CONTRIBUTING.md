# Contributing to PrayDF

Thanks for taking the time to help. Bug reports, ideas, documentation fixes and code are all welcome.

## Before you start

- Search the [existing issues](https://github.com/arvamadax/PrayDF/issues) first; someone may already be on it.
- For anything larger than a small fix, open an issue to agree on the approach before you write code.
- Never attach private or confidential documents to an issue. If a bug only shows up with a specific PDF, describe it or share a file you are allowed to publish.
- Security problems go through [SECURITY.md](SECURITY.md), not public issues.
- By taking part you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Project layout

| Path | What lives there |
|---|---|
| `extension/` | Browser extension (Manifest V3, plain JavaScript, no build step) |
| `native/` | Windows app in Rust (Win32 + PDFium) |
| `assets/` | Logo, icon and `make-icons.py` |
| `docs/src/` | README source; `README.md` is generated from it |
| `brand/` | Fonts and licences used by the README images |

## Browser extension

```bash
npm install
npm run vendor          # copies pdf.js and mammoth into extension/lib
```

Load `extension/` as an unpacked extension (see the README), then click **Reload** on the extension card after each change.

- Keep it dependency-free at runtime: no framework, no bundler.
- Libraries load only when needed (pdf.js on the first PDF, mammoth on the first DOCX).
- Test in at least one Chromium browser and, for changes to `manifest.json` or `background.js`, in Firefox.

## Windows app

Requirements: Rust with the `x86_64-pc-windows-gnu` host and MinGW-w64 (`windres`) on `PATH`.

```bash
cd native
powershell -File fetch-pdfium.ps1
cargo build --release
cargo test
```

- `cargo build --release` must finish **without warnings**.
- No `unwrap()` on data that comes from a file, a link or the user; show the error in the window instead.
- Anything that parses `praydf:` links belongs in `src/link.rs` as a pure function with tests. Treat every link as hostile input.

## README

`README.md` is generated. Edit `docs/src/README.md`, then:

```bash
pip install fonttools brotli
python docs/render.py
```

Commit the source, the regenerated `README.md` and `docs/assets/md/` together.

## Pull requests

1. Fork the repository and create a branch from `main`.
2. Keep each pull request to one change, with a clear title.
3. Describe what changed, why, and how you tested it.
4. Update `CHANGELOG.md` under **Unreleased** for user-visible changes.

Thank you for making PrayDF better.
