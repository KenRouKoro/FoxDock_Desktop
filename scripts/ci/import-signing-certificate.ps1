#Requires -Version 5.1
<#
  从 Base64 解码 PFX，导入到 LocalMachine\My，并输出证书指纹（供 Tauri / signtool 使用）。
  环境变量：
    WINDOWS_CODE_SIGNING_PFX_BASE64  必填，PFX 的 Base64
    WINDOWS_CODE_SIGNING_PFX_PASSWORD 必填
  可选：
    WINDOWS_CODE_SIGNING_PFX_PATH    若已写入磁盘的 PFX 路径，可跳过 Base64
#>
$ErrorActionPreference = "Stop"

$pfxPath = $env:WINDOWS_CODE_SIGNING_PFX_PATH
if (-not $pfxPath) {
  $b64 = $env:WINDOWS_CODE_SIGNING_PFX_BASE64
  if (-not $b64) { throw "WINDOWS_CODE_SIGNING_PFX_BASE64 or WINDOWS_CODE_SIGNING_PFX_PATH is required." }
  $pfxPath = Join-Path $env:RUNNER_TEMP "foxdock-signing.pfx"
  $bytes = [Convert]::FromBase64String($b64)
  [IO.File]::WriteAllBytes($pfxPath, $bytes)
}

$pwd = $env:WINDOWS_CODE_SIGNING_PFX_PASSWORD
if (-not $pwd) { throw "WINDOWS_CODE_SIGNING_PFX_PASSWORD is required." }

$secure = ConvertTo-SecureString -String $pwd -AsPlainText -Force
$cert = Import-PfxCertificate -FilePath $pfxPath -CertStoreLocation "Cert:\LocalMachine\My" -Password $secure

$thumb = $cert.Thumbprint
Write-Host "Imported certificate thumbprint: $thumb"
$env:CERTIFICATE_THUMBPRINT = $thumb

if ($env:GITHUB_OUTPUT) {
  "CERTIFICATE_THUMBPRINT=$thumb" | Out-File -FilePath $env:GITHUB_OUTPUT -Append -Encoding utf8
}
