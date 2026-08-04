<#
.SYNOPSIS
    Windows EV Code Signing Script for Yntra Platform executables and installers.
.DESCRIPTION
    Signs yntra-ui.exe and YntraPlatform-Setup.exe using signtool.exe with SHA256 and RFC 3161 timestamping.
#>

param (
    [string]$CertPath = $env:EV_CERT_PATH,
    [string]$CertPassword = $env:EV_CERT_PASSWORD,
    [string]$CertSubject = $env:EV_CERT_SUBJECT,
    [string]$TimeStampServer = "http://timestamp.digicert.com"
)

$ErrorActionPreference = "Stop"

Write-Host "===================================================" -ForegroundColor Cyans
Write-Host " Yntra Platform - Windows EV Code Signing Execution" -ForegroundColor Cyan
Write-Host "===================================================" -ForegroundColor Cyan

$TargetExe = "target\release\yntra-ui.exe"
$InstallerExe = "target\release\YntraPlatform-Setup.exe"

$Signtool = Get-Command "signtool.exe" -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source
if (-not $Signtool) {
    # Check standard Windows SDK paths
    $SdkPaths = Get-ChildItem "C:\Program Files (x86)\Windows Kits\10\bin\*\x64\signtool.exe" -ErrorAction SilentlyContinue
    if ($SdkPaths) {
        $Signtool = $SdkPaths[0].FullName
    }
}

if (-not $Signtool) {
    Write-Warning "signtool.exe was not found in PATH or Windows SDK. Skipping code signing step."
    Write-Host "Installer created successfully without signature." -ForegroundColor Yellow
    exit 0
}

Write-Host "Using SignTool at: $Signtool" -ForegroundColor Green

function Sign-File ([string]$FilePath) {
    if (-not (Test-Path $FilePath)) {
        Write-Warning "Target file does not exist: $FilePath"
        return
    }

    Write-Host "Signing $FilePath ..." -ForegroundColor Yellow

    if ($CertPath -and (Test-Path $CertPath)) {
        & $Signtool sign /f $CertPath /p $CertPassword /fd sha256 /tr $TimeStampServer /td sha256 $FilePath
    } elseif ($CertSubject) {
        & $Signtool sign /n "$CertSubject" /fd sha256 /tr $TimeStampServer /td sha256 $FilePath
    } else {
        Write-Warning "No EV Certificate path or subject provided (EV_CERT_PATH / EV_CERT_SUBJECT). Skipping sign for $FilePath."
        return
    }

    if ($LASTEXITCODE -eq 0) {
        Write-Host "Successfully signed: $FilePath" -ForegroundColor Green
    } else {
        Write-Error "Failed to sign file: $FilePath with exit code $LASTEXITCODE"
    }
}

Sign-File -FilePath $TargetExe
Sign-File -FilePath $InstallerExe

Write-Host "Code signing process completed." -ForegroundColor Green
