<#
  Bangun installer MSI PrayDF (Windows x64).

  .\build-msi.ps1                 # build MSI ke .\out\PrayDF-<versi>-x64.msi
  .\build-msi.ps1 -SkipCargoBuild # lewati cargo build --release
  .\build-msi.ps1 -WixPath <dir>  # lokasi WiX Toolset v3 (bila tidak ada di PATH)

  Butuh: Rust (cargo) + MinGW-w64 (windres) + WiX Toolset v3 (candle/light).
#>
[CmdletBinding()]
param(
  [string]$WixPath,
  [switch]$SkipCargoBuild
)

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$InstallerDir = $PSScriptRoot
$NativeRoot = (Resolve-Path (Join-Path $InstallerDir '..')).Path
$RepoRoot = (Resolve-Path (Join-Path $NativeRoot '..')).Path
$OutDir = Join-Path $InstallerDir 'out'
$PayloadDir = Join-Path $NativeRoot 'pdfium'

# ---------- 1. versi dari native/Cargo.toml ----------
$CargoToml = Get-Content (Join-Path $NativeRoot 'Cargo.toml') -Raw
if ($CargoToml -notmatch '(?m)^\s*version\s*=\s*"([^"]+)"') {
  throw "Versi tidak ditemukan di native/Cargo.toml"
}
$Version = $Matches[1]
Write-Host "Versi: $Version"

# ProductCode diturunkan dari versi supaya otomatis berubah saat versi naik.
$md5 = [System.Security.Cryptography.MD5]::Create()
$ProductCode = ([guid]::new($md5.ComputeHash([Text.Encoding]::UTF8.GetBytes("PrayDF|$Version")))).ToString('B').ToUpper()

# ---------- 2. toolchain ----------
function Resolve-Cargo {
  $cmd = Get-Command cargo -ErrorAction SilentlyContinue
  if ($cmd) { return $cmd.Source }
  $fallback = Join-Path $env:USERPROFILE '.cargo\bin\cargo.exe'
  if (Test-Path $fallback) { return $fallback }
  throw "cargo tidak ditemukan. Pasang Rust (https://rustup.rs) atau tambahkan ke PATH."
}

function Resolve-Wix {
  param([string]$Hint)
  if ($Hint) {
    $candle = Get-ChildItem -Path $Hint -Filter 'candle.exe' -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($candle) { return $candle.Directory.FullName }
    throw "candle.exe tidak ditemukan di -WixPath '$Hint'"
  }
  if ($env:WIX_HOME) {
    $candle = Get-ChildItem -Path $env:WIX_HOME -Filter 'candle.exe' -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($candle) { return $candle.Directory.FullName }
  }
  $cmd = Get-Command candle.exe -ErrorAction SilentlyContinue
  if ($cmd) { return (Split-Path -Parent $cmd.Source) }
  throw "WiX Toolset v3 tidak ditemukan. Unduh wix314-binaries.zip, ekstrak, lalu jalankan dengan -WixPath <dir> (atau set WIX_HOME)."
}

# ---------- 3. cargo build --release ----------
$Exe = Join-Path $NativeRoot 'target\release\praydf.exe'
if (-not $SkipCargoBuild) {
  $cargo = Resolve-Cargo
  Write-Host "cargo build --release ($cargo)"
  Push-Location $NativeRoot
  try {
    # Jalankan lewat cmd supaya output stderr cargo (progress) tidak memicu
    # NativeCommandError di PowerShell 5.1, tapi kode keluar tetap terbaca.
    $cmdLine = '"' + $cargo + '" build --release 2>&1'
    $log = & cmd.exe /c $cmdLine | Out-String
    $exit = $LASTEXITCODE
  } finally {
    Pop-Location
  }
  if ($exit -ne 0) {
    Write-Host $log
    throw "cargo build --release gagal (kode $exit)."
  }
  if ($log -match '(?m)^warning:') {
    Write-Host $log
    throw "cargo build menghasilkan warning; perbaiki dulu sebelum membuat MSI."
  }
  if ($log -match '(?m)^error') {
    Write-Host $log
    throw "cargo build menghasilkan error."
  }
  Write-Host $log.Trim()
}

if (-not (Test-Path $Exe)) { throw "Tidak ada $Exe (jalankan cargo build --release)." }

# ---------- 4. berkas payload ----------
$Dll = Join-Path $PayloadDir 'bin\pdfium.dll'
$PdfiumLicense = Join-Path $PayloadDir 'LICENSE'
$LicenseDir = Join-Path $PayloadDir 'licenses'
$RepoLicense = Join-Path $RepoRoot 'LICENSE'
foreach ($f in $Exe, $Dll, $PdfiumLicense, $RepoLicense) {
  if (-not (Test-Path $f)) { throw "Berkas tidak ada: $f" }
}
if (-not (Test-Path $LicenseDir)) { throw "Folder lisensi PDFium tidak ada: $LicenseDir" }

# Semua berkas lisensi harus tercantum di PrayDF.wxs (cegah pembaruan yang terlewat).
$wxsText = Get-Content (Join-Path $InstallerDir 'PrayDF.wxs') -Raw
$missing = Get-ChildItem $LicenseDir -File | Where-Object { $wxsText -notmatch [regex]::Escape("\$($_.Name)") }
if ($missing) {
  throw ("Berkas lisensi berikut belum dicantum di PrayDF.wxs: " + ($missing.Name -join ', '))
}

# ---------- 5. syarat penggunaan (terms.txt) + lisensi MIT (LICENSE) dalam bentuk RTF ----------
# Aturan terms.txt: baris berawalan '#' = judul, '- ' = butir, baris kosong = pemisah paragraf;
# baris berurutan tanpa jeda digabung (LICENSE dibungkus per baris). Karakter non-ASCII di-escape (\uN?).
New-Item -ItemType Directory -Force $OutDir | Out-Null
$TermsTxt = Join-Path $InstallerDir 'terms.txt'
foreach ($f in $TermsTxt, (Join-Path $InstallerDir 'art\banner.bmp'), (Join-Path $InstallerDir 'art\dialog.bmp')) {
  if (-not (Test-Path $f)) { throw "Berkas tidak ada: $f (gambar: python installer\make-art.py)" }
}
$LicenseRtf = Join-Path $OutDir 'PrayDF-license.rtf'

function ConvertTo-RtfText([string]$s) {
  $sb = New-Object System.Text.StringBuilder
  foreach ($ch in $s.ToCharArray()) {
    $c = [int]$ch
    if ($ch -in '\', '{', '}') { [void]$sb.Append('\').Append($ch) }
    elseif ($c -gt 126) { [void]$sb.Append('\u').Append($(if ($c -gt 32767) { $c - 65536 } else { $c })).Append('?') }
    else { [void]$sb.Append($ch) }
  }
  $sb.ToString()
}

$paras = New-Object System.Collections.Generic.List[string]
$firstHeading = $true
$buf = @()
$flush = {
  if ($buf.Count) { $paras.Add('\pard\sa120 ' + (ConvertTo-RtfText ($buf -join ' ')) + '\par') }
  $script:buf = @()
}
$sources = @(Get-Content $TermsTxt -Encoding UTF8) + '' + @(Get-Content $RepoLicense -Encoding UTF8)
foreach ($line in $sources) {
  if ($line.Trim() -eq '') { & $flush; continue }
  if ($line.StartsWith('#')) {
    & $flush
    $size = if ($firstHeading) { '\fs30' } else { '\fs22' }
    $firstHeading = $false
    $paras.Add('\pard\sb200\sa100\b' + $size + ' ' + (ConvertTo-RtfText $line.Substring(1).Trim()) + '\b0\fs18\par')
  } elseif ($line.StartsWith('- ')) {
    & $flush
    $paras.Add('\pard\li340\fi-340\sa80 \bullet\tab ' + (ConvertTo-RtfText $line.Substring(2).Trim()) + '\par')
  } else { $buf += $line.Trim() }
}
& $flush
$rtf = '{\rtf1\ansi\ansicpg1252\deff0{\fonttbl{\f0\fnil\fcharset0 Segoe UI;}}\viewkind4\uc1\f0\fs18' + "`r`n" + ($paras -join "`r`n") + "`r`n}"
[System.IO.File]::WriteAllText($LicenseRtf, $rtf, [System.Text.Encoding]::ASCII)

# ---------- 6. build MSI ----------
$Wix = Resolve-Wix -Hint $WixPath
Write-Host "WiX: $Wix"
$Wxs = Join-Path $InstallerDir 'PrayDF.wxs'
$Obj = Join-Path $OutDir 'PrayDF.wixobj'
$Msi = Join-Path $OutDir "PrayDF-$Version-x64.msi"

$defines = @(
  "-dProductCode=$ProductCode",
  "-dProductVersion=$Version",
  "-dPraydfExe=$Exe",
  "-dPdfiumDll=$Dll",
  "-dPdfiumLicense=$PdfiumLicense",
  "-dLicenseDir=$LicenseDir",
  "-dRepoLicense=$RepoLicense",
  "-dLicenseRtf=$LicenseRtf",
  "-dArtDir=$(Join-Path $InstallerDir 'art')"
)

$candle = Join-Path $Wix 'candle.exe'
$light = Join-Path $Wix 'light.exe'
$uiExt = Join-Path $Wix 'WixUIExtension.dll'

Write-Host "candle -> $Obj"
& $candle -nologo -arch x64 -out $Obj @defines $Wxs
if ($LASTEXITCODE -ne 0) { throw "candle gagal (kode $LASTEXITCODE)" }

Write-Host "light  -> $Msi"
& $light -nologo -sice:ICE57 -ext $uiExt -loc (Join-Path $InstallerDir 'PrayDF.wxl') -out $Msi $Obj
if ($LASTEXITCODE -ne 0) { throw "light gagal (kode $LASTEXITCODE)" }

$size = (Get-Item $Msi).Length
Write-Host ""
Write-Host "Selesai: $Msi ($([math]::Round($size / 1MB, 2)) MB)"
Write-Host "ProductCode: $ProductCode"
Write-Host "UpgradeCode: {A1DDDA63-3DFD-45B8-A466-9DADD546A5FF}"
