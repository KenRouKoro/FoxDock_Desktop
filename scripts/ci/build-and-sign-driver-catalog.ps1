#Requires -Version 5.1
<#
  在 src-tauri/windows-driver 下：为 INF 添加 CatalogFile=slime_smol.cat（若尚无）、Inf2Cat 生成目录包、signtool 签名 .cat。
  若找不到 Inf2Cat：跳过（退出 0），不修改 INF，便于未装 WDK 的 CI 仍能完成应用签名构建。
#>
param(
  [string] $RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path,
  [string] $Thumbprint = $env:CERTIFICATE_THUMBPRINT,
  [string] $Inf2CatExe = $env:INF2CAT_EXE,
  [string] $TimestampUrl = "http://timestamp.digicert.com"
)

$ErrorActionPreference = "Stop"
$driverDir = Join-Path $RepoRoot "src-tauri\windows-driver"

if (-not (Test-Path $driverDir)) { throw "Missing directory: $driverDir" }

function Find-Inf2Cat {
  if ($Inf2CatExe -and (Test-Path $Inf2CatExe)) { return $Inf2CatExe }
  $roots = @(
    "${env:ProgramFiles(x86)}\Windows Kits\10\bin",
    "${env:ProgramFiles}\Windows Kits\10\bin"
  )
  foreach ($r in $roots) {
    if (-not (Test-Path $r)) { continue }
    $hit = Get-ChildItem -Path $r -Filter "Inf2Cat.exe" -Recurse -ErrorAction SilentlyContinue |
      Select-Object -First 1 -ExpandProperty FullName
    if ($hit) { return $hit }
  }
  return $null
}

function Add-CatalogFileLine {
  param([string] $Path)
  $text = Get-Content -Path $Path -Raw -Encoding utf8
  if ($text -match '(?m)^CatalogFile=') { return $false }
  if ($text -notmatch '(?m)^DriverVer=') { throw "No DriverVer= in $Path" }
  $text = $text -replace '(?m)^(DriverVer=.*\r?\n)', "`$1CatalogFile=slime_smol.cat`r`n"
  Set-Content -Path $Path -Value $text -Encoding utf8 -NoNewline
  return $true
}

$infNames = @("slime_smol_tracker.inf", "slime_smol_receiver.inf")
$inf2cat = Find-Inf2Cat
if (-not $inf2cat) {
  Write-Warning "Inf2Cat.exe not found (install WDK or set INF2CAT_EXE). Skipping driver catalog; app will still be signed if configured."
  exit 0
}

foreach ($name in $infNames) {
  $p = Join-Path $driverDir $name
  if (Add-CatalogFileLine -Path $p) { Write-Host "Added CatalogFile to $name" }
}

Write-Host "Using Inf2Cat: $inf2cat"
& $inf2cat /driver:"$driverDir" /os:10_NI_X64,10_NI_X86,10_X64,10_X86
if ($LASTEXITCODE -ne 0) { throw "Inf2Cat failed with exit code $LASTEXITCODE" }

$cat = Join-Path $driverDir "slime_smol.cat"
if (-not (Test-Path $cat)) { throw "Expected catalog not found: $cat" }

$signtoolPath = $null
$signtool = Get-Command signtool.exe -ErrorAction SilentlyContinue
if ($signtool) {
  $signtoolPath = $signtool.Source
} else {
  $kits = "${env:ProgramFiles(x86)}\Windows Kits\10\bin"
  $signtoolPath = Get-ChildItem -Path $kits -Filter "signtool.exe" -Recurse -ErrorAction SilentlyContinue |
    Where-Object { $_.FullName -match '\\x64\\' } | Select-Object -First 1 -ExpandProperty FullName
}

if (-not $signtoolPath) { throw "signtool.exe not found. Install Windows SDK signing tools." }
if (-not $Thumbprint) { throw "CERTIFICATE_THUMBPRINT is required for signing .cat" }

Write-Host "Signing catalog with thumbprint $Thumbprint"
& $signtoolPath sign /fd SHA256 /sha1 $Thumbprint /tr $TimestampUrl /td SHA256 /v $cat
if ($LASTEXITCODE -ne 0) { throw "signtool failed signing .cat" }

Write-Host "Driver catalog ready: $cat"
