# Code Signing Script for Windows Executables
# Usage: .\sign.ps1 <path-to-exe>

param(
    [Parameter(Mandatory=$true)]
    [string]$ExePath,
    
    [string]$CertPath = $env:CODE_SIGNING_CERT_PATH,
    [string]$CertPassword = $env:CODE_SIGNING_CERT_PASSWORD
)

# Check if signtool is available
$signtool = Get-Command signtool.exe -ErrorAction SilentlyContinue
if (-not $signtool) {
    # Try to find in Windows SDK
    $sdkPaths = @(
        "C:\Program Files (x86)\Windows Kits\10\bin\*\x64\signtool.exe",
        "C:\Program Files\Windows Kits\10\bin\*\x64\signtool.exe"
    )
    foreach ($path in $sdkPaths) {
        $found = Get-Item $path -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($found) {
            $signtool = $found.FullName
            break
        }
    }
    if (-not $signtool) {
        Write-Error "signtool.exe not found. Install Windows SDK."
        exit 1
    }
} else {
    $signtool = $signtool.Source
}

# Check certificate
if (-not $CertPath -or -not $CertPassword) {
    Write-Error "Set CODE_SIGNING_CERT_PATH and CODE_SIGNING_CERT_PASSWORD environment variables"
    exit 1
}

if (-not (Test-Path $CertPath)) {
    Write-Error "Certificate not found: $CertPath"
    exit 1
}

if (-not (Test-Path $ExePath)) {
    Write-Error "Executable not found: $ExePath"
    exit 1
}

Write-Host "Signing: $ExePath"

# Sign the executable
& $signtool sign `
    /f $CertPath `
    /p $CertPassword `
    /tr http://timestamp.digicert.com `
    /td sha256 `
    /d "R Teams" `
    /du "https://github.com/thichcode/rust_teams" `
    $ExePath

if ($LASTEXITCODE -eq 0) {
    Write-Host "✅ Signed successfully: $ExePath" -ForegroundColor Green
    
    # Verify signature
    & $signtool verify /pa $ExePath
    if ($LASTEXITCODE -eq 0) {
        Write-Host "✅ Signature verified" -ForegroundColor Green
    } else {
        Write-Warning "Signature verification failed"
    }
} else {
    Write-Error "Signing failed with exit code: $LASTEXITCODE"
    exit 1
}
