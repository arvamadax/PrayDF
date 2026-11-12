<p align="center"><img src="assets/logo.svg" alt="PrayDF" width="440"></p>

Viewer dokumen ringan: extension Brave/Chrome (PDF, DOCX, teks, gambar) plus aplikasi PDF desktop untuk Windows. File tidak pernah diunggah ke mana pun.

## Unduh

Ambil versi siap pakai di halaman [Releases](../../releases/latest):

- `PrayDF-windows-x64.zip`: `praydf.exe` + `pdfium.dll`. Ekstrak ke satu folder, lalu jalankan `praydf.exe --register` supaya muncul di "Open with" untuk `.pdf` dan bisa dipanggil dari extension.
- `PrayDF-extension.zip`: ekstrak, lalu muat lewat langkah di bawah.

## Pasang di Brave

1. Buka `brave://extensions`.
2. Nyalakan **Developer mode** (pojok kanan atas).
3. Klik **Load unpacked**, pilih folder `extension/` (bukan root proyek).
4. Klik ikon extension di toolbar. Viewer terbuka di tab baru. Seret file ke sana atau klik **Buka file**.

Setelah mengubah kode, klik tombol reload pada kartu extension di `brave://extensions`.

Tab yang menerima PDF (misalnya tombol **Cetak** di Google Classroom, atau link PDF biasa) otomatis dibuka di viewer PrayDF, seperti viewer PDF bawaan Chrome. PDF yang dibuka lewat alamat `blob:` atau dari disk (`file://`) belum ikut dialihkan.

## Format

| Format | Cara | Catatan |
|---|---|---|
| PDF | pdf.js (Mozilla) | Tampilan ala viewer PDF Google: thumbnail di samping, nomor halaman bisa diketik, zoom, pas lebar/halaman, putar, cari (Ctrl+F), unduh, cetak. Link aktif, PDF berpassword ditanya passwordnya |
| DOCX | mammoth.js | Fokus ke isi (judul, teks, list, tabel, gambar), bukan tata letak persis seperti Word |
| TXT, MD, CSV, JSON, LOG, XML, HTML | ditampilkan sebagai teks | HTML tidak dijalankan |
| PNG, JPG, GIF, WEBP, SVG | `<img>` bawaan browser | |

## Kenapa ringan

- Tanpa framework, tanpa build step. Kode viewer sendiri sekitar 150 baris.
- Library dimuat **hanya saat dibutuhkan**: pdf.js (~2 MB) saat PDF pertama dibuka, mammoth (~400 KB) saat DOCX pertama dibuka. Membuka teks atau gambar tidak memuat keduanya.
- PDF dirender per halaman yang terlihat saja (bawaan `PDFViewer` pdf.js), jadi PDF ratusan halaman tetap ringan.
- Dokumen sebelumnya dibuang dari memori saat membuka file baru.

## Aplikasi desktop PrayDF (opsional)

Viewer PDF native (Rust + PDFium) di `native/`. Lebih hemat RAM (~22 MB) dan terbuka dalam ~40 ms. Cara build dan pasang ada di [native/README.md](native/README.md).

Setelah `praydf.exe --register`, extension bisa mengoper PDF ke aplikasi:
- **Klik kanan link PDF** di web → "Buka di aplikasi PrayDF". Aplikasi mengunduh sendiri file-nya.
- **Tombol "Buka di aplikasi"** di toolbar viewer saat PDF terbuka. File disimpan dulu ke `Downloads\PrayDF\`, lalu path-nya dikirim ke aplikasi.

Brave akan bertanya "Buka PrayDF?" sebelum menjalankan aplikasi. Aplikasi hanya menerima link `http(s)` dan path lokal `.pdf`; lainnya ditolak.

## Struktur

```
extension/          <- folder yang di-load ke Brave
  manifest.json     tombol toolbar, izin contextMenus + downloads + webRequest
  background.js     buka viewer.html; alihkan tab PDF ke viewer; menu klik kanan "Buka di aplikasi PrayDF"
  viewer.html/css/js
  icons/            ikon PrayDF (dibuat oleh assets/make-icons.py)
  lib/              pdf.js + mammoth (disalin oleh npm run vendor, tidak di-commit)
native/             aplikasi desktop PrayDF (Rust)
assets/             logo, ikon, dan skrip pembuat ikon
vendor.mjs          salin library dari node_modules ke extension/lib
```

Dari kode sumber, isi `extension/lib` dulu dengan `npm install` lalu `npm run vendor`.

## Update library

```bash
npm update
npm run vendor
```

## Ide berikutnya (belum dibuat)

- XLSX lewat SheetJS, PPTX (sulit, tidak ada library ringan yang bagus).
- Ingat posisi halaman terakhir per file.
- DOCX/TXT di aplikasi PrayDF.

## Lisensi

[MIT](LICENSE). Library pihak ketiga memakai lisensinya sendiri: pdf.js (Apache-2.0), mammoth (BSD-2-Clause), PDFium (BSD-3-Clause/Apache-2.0, disertakan di zip rilis bersama `pdfium.dll`).
