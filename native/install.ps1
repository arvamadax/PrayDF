# Pasang PrayDF untuk user ini (tanpa admin): salin exe + dll, daftarkan .pdf dan praydf:, buat shortcut Start Menu.
# Copot: & "$env:LOCALAPPDATA\PrayDF\praydf.exe" --unregister; lalu hapus folder dan shortcut-nya.
$ErrorActionPreference = 'Stop'
$exe = Join-Path $PSScriptRoot 'target\release\praydf.exe'
$dll = Join-Path $PSScriptRoot 'pdfium\bin\pdfium.dll'
foreach ($f in $exe, $dll) { if (-not (Test-Path $f)) { throw "Tidak ada: $f (jalankan fetch-pdfium.ps1 dan cargo build --release dulu)" } }
$dest = Join-Path $env:LOCALAPPDATA 'PrayDF'
Get-Process praydf -ErrorAction SilentlyContinue | Where-Object { $_.Path -like "$dest*" } | Stop-Process
New-Item -ItemType Directory -Force $dest | Out-Null
Copy-Item $exe, $dll $dest -Force
$app = Join-Path $dest 'praydf.exe'
$r = Start-Process $app -ArgumentList '--register' -Wait -NoNewWindow -PassThru
if ($r.ExitCode -ne 0) { throw "Pendaftaran gagal (kode $($r.ExitCode))" }
$lnk = Join-Path ([Environment]::GetFolderPath('Programs')) 'PrayDF.lnk'
$s = (New-Object -ComObject WScript.Shell).CreateShortcut($lnk)
$s.TargetPath = $app; $s.WorkingDirectory = $dest; $s.Save()
"Terpasang di $dest"
