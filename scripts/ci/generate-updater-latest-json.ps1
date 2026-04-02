$ErrorActionPreference = "Stop"

$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
$bundleRoot = Join-Path $repoRoot "src-tauri\target\release\bundle"
$tauriConfigPath = Join-Path $repoRoot "src-tauri\tauri.conf.json"

if (-not $env:FOXDOCK_RELEASE_TAG) {
  throw "FOXDOCK_RELEASE_TAG is required to generate latest.json."
}

if (-not $env:GITHUB_REPOSITORY) {
  throw "GITHUB_REPOSITORY is required to generate latest.json."
}

if (-not (Test-Path $bundleRoot)) {
  throw "Bundle directory not found: $bundleRoot"
}

$tauriConfig = Get-Content $tauriConfigPath -Raw | ConvertFrom-Json
$version = [string]$tauriConfig.version
if (-not $version) {
  throw "Could not read version from $tauriConfigPath"
}

function Get-UpdaterAssetPair {
  param(
    [string]$Directory,
    [string]$Pattern
  )

  if (-not (Test-Path $Directory)) {
    return $null
  }

  $asset = Get-ChildItem -Path $Directory -Filter $Pattern -File |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
  if (-not $asset) {
    return $null
  }

  $sigPath = "$($asset.FullName).sig"
  if (-not (Test-Path $sigPath)) {
    throw "Signature file not found for updater asset: $sigPath"
  }

  return @{
    Asset = $asset
    SigPath = $sigPath
  }
}

$assetPair = Get-UpdaterAssetPair -Directory (Join-Path $bundleRoot "nsis") -Pattern "*.exe"
if (-not $assetPair) {
  $assetPair = Get-UpdaterAssetPair -Directory (Join-Path $bundleRoot "msi") -Pattern "*.msi"
}
if (-not $assetPair) {
  throw "No signed NSIS or MSI updater asset was found under $bundleRoot"
}

$signature = (Get-Content $assetPair.SigPath -Raw).Trim()
if (-not $signature) {
  throw "Signature file is empty: $($assetPair.SigPath)"
}

$assetName = $assetPair.Asset.Name
$releaseTag = $env:FOXDOCK_RELEASE_TAG.Trim()
$releaseNotes = $env:GITHUB_RELEASE_BODY
$publishedAt = $env:GITHUB_RELEASE_PUBLISHED_AT
if (-not $publishedAt) {
  $publishedAt = (Get-Date).ToUniversalTime().ToString("o")
}

$latestJson = @{
  version = $version
  notes = if ($releaseNotes) { $releaseNotes } else { "" }
  pub_date = $publishedAt
  platforms = @{
    "windows-x86_64" = @{
      signature = $signature
      url = "https://github.com/$($env:GITHUB_REPOSITORY)/releases/download/$releaseTag/$assetName"
    }
  }
}

$latestJsonPath = Join-Path $bundleRoot "latest.json"
$latestJson | ConvertTo-Json -Depth 10 | Set-Content -Path $latestJsonPath -Encoding utf8
Write-Host "Generated updater metadata: $latestJsonPath"
