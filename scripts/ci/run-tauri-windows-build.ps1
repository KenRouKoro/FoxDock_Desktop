#Requires -Version 5.1
<#
  CI 入口：若设置了 WINDOWS_CODE_SIGNING_PFX_BASE64 + WINDOWS_CODE_SIGNING_PFX_PASSWORD，
  则导入证书、可选 Inf2Cat+签名驱动目录包、再带 certificateThumbprint 执行 pnpm tauri build。
  未配置密钥时仅执行 pnpm tauri build。
#>
$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..\..")).Path
Set-Location $repoRoot

$pfxB64 = $env:WINDOWS_CODE_SIGNING_PFX_BASE64
$pfxPwd = $env:WINDOWS_CODE_SIGNING_PFX_PASSWORD

if ($pfxB64 -and $pfxPwd) {
  Write-Host "Code signing: importing PFX and preparing build..."
  & "$PSScriptRoot\import-signing-certificate.ps1"
  $thumb = $env:CERTIFICATE_THUMBPRINT
  if (-not $thumb) { throw "CERTIFICATE_THUMBPRINT not set after import." }

  $env:CERTIFICATE_THUMBPRINT = $thumb
  & "$PSScriptRoot\build-and-sign-driver-catalog.ps1" -RepoRoot $repoRoot -Thumbprint $thumb

  $timestampUrl = "http://timestamp.digicert.com"
  $json = @{
    bundle = @{
      windows = @{
        certificateThumbprint = $thumb
        digestAlgorithm       = "sha256"
        timestampUrl          = $timestampUrl
      }
    }
  } | ConvertTo-Json -Compress -Depth 5

  $cfgDir = if ($env:RUNNER_TEMP) { $env:RUNNER_TEMP } else { Join-Path $repoRoot "target" }
  New-Item -ItemType Directory -Force -Path $cfgDir | Out-Null
  $cfgFile = Join-Path $cfgDir "tauri-signing-config.json"
  Set-Content -Path $cfgFile -Value $json -Encoding utf8
  Write-Host "Running: pnpm tauri build --config $cfgFile"
  pnpm tauri build --config $cfgFile
} else {
  Write-Host "No WINDOWS_CODE_SIGNING_PFX_BASE64 / PASSWORD — building without Authenticode signing."
  pnpm tauri build
}
