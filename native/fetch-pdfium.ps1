# Unduh pdfium.dll (bblanchon/pdfium-binaries) ke native/pdfium/.
$ErrorActionPreference = 'Stop'
$dest = Join-Path $PSScriptRoot 'pdfium'
$tgz = Join-Path $env:TEMP 'pdfium-win-x64.tgz'
$url = 'https://github.com/bblanchon/pdfium-binaries/releases/latest/download/pdfium-win-x64.tgz'
New-Item -ItemType Directory -Force $dest | Out-Null
Invoke-WebRequest $url -OutFile $tgz -UseBasicParsing
tar -xzf $tgz -C $dest
Remove-Item $tgz
Get-Item (Join-Path $dest 'bin\pdfium.dll') | Select-Object FullName, Length
