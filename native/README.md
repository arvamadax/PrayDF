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

Skrip ini menyalin `praydf.exe` + `pdfium.dll` ke `%LOCALAPPDATA%\PrayDF\`, menjalankan `--register`, dan membuat shortcut di Start Menu. Jalankan lagi setelah build baru. Untuk pengguna akhir, pakai installer MSI (lihat bagian "Installer MSI").

Copot: `& "$env:LOCALAPPDATA\PrayDF\praydf.exe" --unregister`, lalu hapus folder itu dan shortcut `PrayDF` di Start Menu.

## Daftarkan untuk .pdf (tanpa admin, HKCU)

```powershell
praydf.exe --register      # ProgID PrayDF.PDF + "Open with" untuk .pdf + Default apps
praydf.exe --unregister    # hapus lagi
```

`--register` juga menulis `HKCU\Software\PrayDF\Capabilities` dan nilai `PrayDF` di `HKCU\Software\RegisteredApplications`, supaya PrayDF muncul di **Settings > Apps > Default apps**. `Capabilities` menunjuk `.pdf` ke `PrayDF.PDF` dan `praydf:` ke protocol `praydf`.

Lalu pilih PrayDF lewat klik kanan PDF → Open with → Choose another app.
Jalankan `--register` lagi setelah memindahkan exe.

## Installer MSI

Sumber installer ada di `installer\`:

| Berkas | Isi |
|---|---|
| `installer\PrayDF.wxs` | Paket WiX v3: dual-purpose (per-user atau per-machine), registry HKMU, shortcut Start Menu, dialog selamat datang, syarat penggunaan, dan cakupan |
| `installer\PrayDF.wxl` | Menimpa dua string dialog "Setup closed" bawaan WixUI (dipakai saat instalasi diserahkan ke jendela ter-elevasi) |
| `installer\terms.txt` | Isi syarat penggunaan (judul `#`, butir `- `). Build menambahkan teks `LICENSE` di bawahnya |
| `installer\art\` | `banner.bmp` dan `dialog.bmp` hasil `installer\make-art.py` (Pillow), sudah di-commit |
| `installer\build-msi.ps1` | Build `installer\out\PrayDF-<versi>-x64.msi` |

Butuh WiX Toolset v3 (candle.exe + light.exe) selain Rust dan MinGW-w64:

```powershell
.\installer\build-msi.ps1 -WixPath "C:\Program Files (x86)\WiX Toolset v3.14\bin"
# atau set WIX_HOME, atau taruh candle.exe di PATH
```

Skrip menjalankan `cargo build --release`, membaca versi dari `Cargo.toml`, lalu membuat MSI di `installer\out\`. Semua entri registry ditulis lewat tabel Registry dengan root `HKMU`, jadi otomatis ke HKCU untuk per-user dan HKLM untuk per-machine.

Catatan penting:

- **UpgradeCode (tetap selamanya):** `{A1DDDA63-3DFD-45B8-A466-9DADD546A5FF}`. Jangan diubah, atau versi baru tidak akan menggantikan versi lama.
- ProductCode dihitung dari versi, sehingga naik versi otomatis menghasilkan installer terpisah.
- Installer berbahasa Inggris. Alur dialog: Welcome, Terms of use (kotak "I accept" wajib), Scope, lalu progres dan selesai. Semua dialog tampil sebelum `CostInitialize`, supaya folder dihitung memakai `ALLUSERS` dan `MSIINSTALLPERUSER` final.
- **Per-user** (`ALLUSERS=2`, `MSIINSTALLPERUSER=1`) tanpa admin. **Semua pengguna**: paket bertanda `InstallPrivileges="limited"`, jadi Windows Installer tidak meminta elevasi sendiri dan instalasi per-machine dari proses biasa gagal dengan error 1925. Tombol Install berperisai (`RelaunchElevated`) menjalankan ulang `msiexec` lewat `Start-Process -Verb RunAs` dengan `ALLUSERS=1 MSIINSTALLPERUSER=""` (UI dasar, log di `%TEMP%\PrayDF-setup.log`). `MSIINSTALLPERUSER` harus dikosongkan, kalau tidak folder tetap dialihkan ke `%LOCALAPPDATA%\Programs` sementara registrasi masuk HKLM. Kedua properti ditandai `Secure="yes"`.
- Target `CustomAction` dibatasi 255 karakter (ICE03). Kalau memanjang, perintah terpotong diam-diam; tetap periksa warning `LGHT1076`.
- Pintasan Start Menu memakai `ProgramMenuFolder` (otomatis ke Start Menu bersama saat `ALLUSERS=1`). `CommonProgramsFolder` bukan folder Windows Installer dan menjadi `C:\`. `build-msi.ps1` menekan ICE57 (`-sice:ICE57`) karena satu komponen melayani dua cakupan.
- Jangan pakai `RemoveRegistryKey Action="removeOnUninstall"` di komponen registrasi: WiX 3 menulisnya sebagai baris Registry `Name = '-'`, dan Windows Installer sendiri sudah menghapus nilai yang ia tulis saat uninstall.
- Dua tombol Install (berperisai dan biasa) bertumpuk. Keduanya harus di-`disable` selain di `show`/`hide`, dan tiap `Publish` dijaga kondisi cakupannya, karena Enter dan Alt+I tetap mencapai kontrol tersembunyi.
- Pengujian tanpa UI: per-machine `msiexec /i <msi> ALLUSERS=1 MSIINSTALLPERUSER="" PRAYDF_TERMS_ACCEPTED=1 /qb` dari proses ter-elevasi; per-user `msiexec /i <msi> PRAYDF_TERMS_ACCEPTED=1 /qn`. Mencopot produk per-machine juga butuh proses ter-elevasi.
- Belum terselesaikan: pada pengujian per-user, kunci `HKCU\Software\Classes\...` (ProgID `.pdf`, `praydf:`) tidak terlihat setelah instalasi, padahal `Software\PrayDF\Capabilities` dan `RegisteredApplications` terlihat. Jalur per-machine (HKLM) normal. Periksa di sesi pengguna biasa sebelum rilis.
- Judul gaya teks memakai sintaks `{\WixUI_Font_Title}` (dengan backslash). Tanpa backslash, Windows Installer menampilkan teks `{WixUI_Font_Title}` apa adanya.
- Teks lisensi dibuat `build-msi.ps1` dari `terms.txt` + `LICENSE`. Jangan menulis header RTF dengan string PowerShell berbacktick (`` `r `` dan `` `f `` menjadi karakter kontrol).
- Installer belum ditandatangani digital, jadi Windows SmartScreen bisa menampilkan peringatan "Windows protected your PC".
- `installer\out\` tidak ikut masuk ke git.

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
