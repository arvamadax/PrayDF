# Changelog

All notable changes to PrayDF are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Windows installer `PrayDF-<version>-x64.msi`: choose the install scope (per-user without administrator rights, or per-machine for all users), get a Start Menu shortcut, and register PrayDF in Windows Default apps.
- The installer is in English and walks through Welcome, terms of use (summary plus the MIT licence, with an accept checkbox), install scope, then progress. It carries PrayDF branding on its side panel and banner.
- "For all users" installs work: the Install button relaunches the installer elevated (UAC) because the package is marked for limited-privilege per-user installs. The Start Menu shortcut now goes to the shared Start Menu instead of the drive root.
- The installer leaves an "Open Default apps settings" checkbox on its last page, which opens the Default apps page in Windows Settings so you can pick PrayDF for `.pdf` yourself.
- The Windows app now registers itself in Settings > Apps > Default apps (`praydf.exe --register` and `--unregister`).
- Firefox 128+ support for the browser extension, alongside Chrome, Edge, Brave, Opera and Vivaldi.
- A branded README with screenshots, an architecture overview and measured performance.
- Contributing guide, security policy, code of conduct, and issue and pull request templates.

### Changed
- The README now installs the Windows app with the MSI; the zip stays available as a portable alternative.

## [0.1.0] - 2026-11-13

### Added
- Browser extension: viewer for PDF (pdf.js), DOCX (mammoth), text and images with thumbnails, page jump, zoom, fit, rotate, search, print and download.
- Browser extension: tabs that receive a PDF open in the PrayDF viewer automatically.
- Browser extension: "Open in app" hands the current PDF to the Windows app.
- Windows app (Rust + Win32 + PDFium): thumbnails, page jump, zoom, fit to width or page, rotate, search with highlights, print and save a copy.
- Windows app: `--register` and `--unregister` for "Open with" and the `praydf:` link, without administrator rights.
- Windows app: `praydf:` links accept only http(s) URLs and local `.pdf` files; downloads run in the background.
- PrayDF logo and application icon.

[Unreleased]: https://github.com/arvamadax/PrayDF/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/arvamadax/PrayDF/releases/tag/v0.1.0
