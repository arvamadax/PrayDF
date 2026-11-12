# PrayDF (native Windows)

Viewer PDF ringan: Rust + Win32 + PDFium. Tampilan gelap ala viewer PDF Chrome.

## Build

Butuh Rust dengan host `x86_64-pc-windows-gnu` (tanpa Visual Studio).

```powershell
.\fetch-pdfium.ps1          # unduh pdfium.dll ke native\pdfium\bin\
cargo build --release       # hasil: target\release\praydf.exe
cargo test
```

Dari `target\release`, exe otomatis memakai `..\..\pdfium\bin\pdfium.dll`.

## Pasang

```powershell
.\install.ps1
```

Skrip ini menyalin `praydf.exe` + `pdfium.dll` ke `%LOCALAPPDATA%\PrayDF\`, menjalankan `--register`, dan membuat shortcut di Start Menu. Jalankan lagi setelah build baru.

Copot: `& "$env:LOCALAPPDATA\PrayDF\praydf.exe" --unregister`, lalu hapus folder itu dan shortcut `PrayDF` di Start Menu.

## Daftarkan untuk .pdf (tanpa admin, HKCU)

```powershell
praydf.exe --register      # ProgID PrayDF.PDF + "Open with" untuk .pdf
praydf.exe --unregister    # hapus lagi
```

Lalu pilih PrayDF lewat klik kanan PDF → Open with → Choose another app.
Jalankan `--register` lagi setelah memindahkan exe.

## Protocol `praydf:` (dari extension)

`--register` juga mendaftarkan `HKCU\Software\Classes\praydf`. Format: `praydf:` + `encodeURIComponent(target)`.

- Diterima: `http(s)://...` (diunduh ke `%TEMP%\praydf\`, nama disanitasi, berakhiran `.pdf`), path lokal `C:\...\x.pdf`, atau `file:///C:/.../x.pdf`.
- Ditolak dengan MessageBox: UNC (`\\server\...`, `file://host/...`), path relatif, ekstensi selain .pdf, skema lain, karakter kontrol, dan tautan lebih dari 4096 karakter.

## Pakai

| Aksi | Cara |
|---|---|
| Buka | argumen, Ctrl+O, tombol folder, atau seret file |
| Halaman | ketik nomor + Enter, klik thumbnail, PageUp/PageDown, Home/End |
| Zoom | tombol −/+, Ctrl+scroll, Ctrl+plus/minus; tombol pas atau Ctrl+0 = pas halaman ↔ pas lebar |
| Putar | tombol putar (90° searah jarum jam) |
| Cari | Ctrl+F, Enter/F3 berikutnya, Shift+Enter sebelumnya, Esc tutup |
| Simpan salinan | Ctrl+S |
| Cetak | Ctrl+P |
