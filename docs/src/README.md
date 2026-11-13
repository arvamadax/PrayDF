<p align="center"><img src="docs/assets/banner.svg" alt="PrayDF: fast, private document viewer for your browser and Windows" width="100%"></p>

<p align="center"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-4f46e5"> <img alt="Latest release" src="https://img.shields.io/github/v/release/arvamadax/PrayDF?color=4f46e5"> <img alt="Platforms" src="https://img.shields.io/badge/platform-Windows%20%7C%20Chrome%20%7C%20Edge%20%7C%20Brave%20%7C%20Firefox-1e1b3a"> <img alt="Built with Rust" src="https://img.shields.io/badge/built%20with-Rust%20%2B%20PDFium-1e1b3a"></p>

# PrayDF

PrayDF is a lightweight, open-source document viewer. Open PDFs, Word documents, text and images right in your browser, or let the native Windows app open a PDF in a fraction of a second. Your files never leave your device.

<p align="center"><a href="https://github.com/arvamadax/PrayDF/releases/latest">Download</a> · <a href="#install">Install</a> · <a href="#build-from-source">Build from source</a> · <a href="CONTRIBUTING.md">Contributing</a> · <a href="CHANGELOG.md">Changelog</a></p>

## Why PrayDF

- **Fast.** The Windows app opens a PDF in about 0.2 s and needs about 22 MB of RAM with one document open (see Performance).
- **Private.** Documents are rendered on your device. PrayDF uploads nothing and has no telemetry.
- **Familiar.** A dark, Chrome-style viewer: page thumbnails, type a page number, zoom, fit to width or page, rotate, search, print and save.
- **Open.** MIT licensed and built on proven engines: pdf.js, mammoth and PDFium.

## Features

| Feature | Browser extension | Windows app |
|---|---|---|
| PDF | Yes, with pdf.js | Yes, with PDFium |
| Word documents (.docx) | Yes, with mammoth | Planned |
| Text, Markdown, CSV, JSON, XML | Yes | Planned |
| Images (PNG, JPG, GIF, WebP, SVG) | Yes | Not planned |
| Thumbnails, page jump, zoom, fit, rotate | Yes | Yes |
| Search with highlights (Ctrl+F) | Yes | Yes |
| Print and save a copy | Yes | Yes |
| Opens PDF tabs in PrayDF automatically | Yes | Not applicable |
| Hand a PDF to the desktop app | Yes, "Open in app" | Receives it |

## Screenshots

The Windows app (top) and the browser extension (bottom) searching the same 120-page lecture PDF for "regresi": 3 of 12 matches, highlighted in place, with page thumbnails on the left.

<p align="center"><img src="docs/assets/screenshot-windows.png" alt="PrayDF for Windows showing page 47 with the search box reading 3 of 12" width="100%"></p>
<p align="center"><img src="docs/assets/screenshot-extension.png" alt="PrayDF browser extension showing page 47 with highlighted search matches" width="100%"></p>

## Install

### Windows app (Windows 10 and 11, 64-bit)

1. Download `PrayDF-<version>-x64.msi` from the [latest release](https://github.com/arvamadax/PrayDF/releases/latest).
2. Start the installer. Read the terms of use (a short summary plus the MIT licence), tick **I accept the terms of use above**, and press **Next**.
3. Pick the install scope, then press **Install**:
   - **Only for me** installs into `%LOCALAPPDATA%\Programs\PrayDF` and needs no administrator rights.
   - **For all users of this computer** installs into `%ProgramFiles%\PrayDF`. The Install button shows a shield: Windows asks for administrator permission and the installation continues in a new, elevated window.
4. The installer adds a **PrayDF** shortcut to the Start menu and registers PrayDF for PDF files. On the last screen of a per-user install, leave **Open Default apps settings** ticked to jump straight to the Default apps page in Windows Settings, where you can pick PrayDF as the app for `.pdf`. Windows keeps that choice in your hands: the installer never sets a default silently.

> The installer is not signed yet, so Windows SmartScreen may warn that it protected your PC. Choose **More info → Run anyway** when you trust the download.

**Portable alternative.** `PrayDF-windows-x64.zip` contains `praydf.exe` and `pdfium.dll`. Extract both into one folder and run `praydf.exe --register` once: PrayDF then appears under **Open with** for PDF files and can receive files from the extension, all without administrator rights. Run `praydf.exe --unregister` to remove it again.

### Browser extension

Download `PrayDF-extension.zip` from the same release and extract it.

| Browser | How to load it |
|---|---|
| Chrome, Edge, Brave, Opera, Vivaldi | Open the extensions page, turn on Developer mode, choose **Load unpacked** and select the `PrayDF-extension` folder |
| Firefox 128 or later | Open `about:debugging`, choose This Firefox, then **Load Temporary Add-on** and pick `manifest.json`. Allow access to all sites in `about:addons` under PrayDF, Permissions |

> Listings on the Chrome Web Store, Edge Add-ons, Firefox Add-ons and the Microsoft Store are on the roadmap.

## Build from source

Requirements: Node.js 18 or later, Rust with the `x86_64-pc-windows-gnu` host (no Visual Studio needed), MinGW-w64 for `windres`, and Python 3 only if you regenerate the icons.

```bash
# Browser extension: copy pdf.js and mammoth into extension/lib
npm install
npm run vendor

# Windows app
cd native
powershell -File fetch-pdfium.ps1   # downloads pdfium.dll into native/pdfium
cargo build --release               # builds target/release/praydf.exe
cargo test                          # runs the unit tests
powershell -File install.ps1        # copies to %LOCALAPPDATA%\PrayDF and registers
powershell -File installer\build-msi.ps1   # builds installer/out/PrayDF-<version>-x64.msi
```

## How it works

The extension watches for tabs that receive a PDF and opens them in its own viewer, the way Chrome's PDF viewer does. From there, **Open in app** hands the file to the Windows app through a `praydf:` link. The app checks every link before it opens anything.

<p align="center"><img src="docs/assets/architecture.svg" alt="How PrayDF works: website tab to PrayDF extension to PrayDF for Windows" width="100%"></p>

## Performance

Measured on Windows 11 with a 12-page PDF. Your numbers will vary with the machine and the document.

| Measurement | Result |
|---|---|
| Download size of the Windows app | 0.97 MB for praydf.exe plus 7.5 MB for pdfium.dll |
| Memory with one PDF open | 22.5 MB working set, 7.9 MB private |
| Time to open a PDF | 31 to 43 ms when warm, 212 ms on the first launch |

## Privacy and security

- Documents are opened and rendered on your device. PrayDF only downloads a document you asked it to open, and sends nothing anywhere.
- The Windows app installs without administrator rights and then registers itself only under `HKEY_CURRENT_USER`; an install for all users registers under `HKEY_LOCAL_MACHINE`.
- `praydf:` links are validated: only `http(s)` URLs and absolute local `.pdf` paths are accepted. Network paths (`\\server\share`, `file://host`), other schemes, relative paths and control characters are refused.
- Found a vulnerability? Please report it privately as described in [SECURITY.md](SECURITY.md).

## Project structure

```
.
├── extension/   Browser extension (Manifest V3): viewer, background worker, icons
├── native/      Windows app in Rust: main.rs (window), doc.rs (PDFium),
│                print.rs (printing), link.rs (praydf: links), installer/ (MSI)
├── assets/      Logo, icon and the script that builds the icons
├── docs/        README source (docs/src) and rendered images
├── brand/       Fonts and licences used by the README images
└── vendor.mjs   Copies pdf.js and mammoth into extension/lib
```

## Roadmap

- Signed releases without a SmartScreen warning, and a Microsoft Store listing
- Chrome Web Store, Edge Add-ons and Firefox Add-ons listings
- Word documents and text files in the Windows app
- Remember the last page of each file

## Contributing

Bug reports, ideas and pull requests are welcome. Read [CONTRIBUTING.md](CONTRIBUTING.md) to set up the project and the [Code of Conduct](CODE_OF_CONDUCT.md) before joining the discussion. Open an [issue](https://github.com/arvamadax/PrayDF/issues) to report a bug or suggest a feature.

## License

PrayDF is released under the [MIT License](LICENSE). It uses pdf.js (Apache-2.0), mammoth (BSD-2-Clause) and PDFium (BSD-3-Clause and Apache-2.0); their licences ship with the release files. The README images use Inter and JetBrains Mono under the SIL Open Font License.
