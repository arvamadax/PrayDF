# Changelog

All notable changes to PrayDF are recorded here. The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and versions follow [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added
- Firefox 128+ support for the browser extension, alongside Chrome, Edge, Brave, Opera and Vivaldi.
- A branded README with screenshots, an architecture overview and measured performance.
- Contributing guide, security policy, code of conduct, and issue and pull request templates.

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
