# Code Signing Setup Guide

## Overview

Code signing makes your application more trustworthy and removes browser warnings like "Keep/Delete" when downloading.

## Option 1: Buy a Code Signing Certificate (Recommended)

### Where to Buy
- **Sectigo** (~$50/year) - https://sectigo.com
- **DigiCert** (~$100/year) - https://digicert.com
- **GlobalSign** (~$80/year) - https://globalsign.com

### Steps
1. Purchase a code signing certificate
2. Generate CSR (Certificate Signing Request)
3. Complete validation (email, domain ownership)
4. Receive .pfx certificate file
5. Store in GitHub Secrets

## Option 2: Self-Signed Certificate (Testing Only)

### Create Self-Signed Certificate
```powershell
# Run in PowerShell as Administrator
$cert = New-SelfSignedCertificate `
    -Type CodeSigningCert `
    -Subject "CN=R Teams" `
    -CertStoreLocation "Cert:\CurrentUser\My" `
    -KeyExportPolicy Exportable `
    -KeySpec Signature `
    -KeyLength 2048 `
    -KeyAlgorithm RSA `
    -HashAlgorithm SHA256 `
    -NotAfter (Get-Date).AddYears(3)

# Export to .pfx file
$password = ConvertTo-SecureString -String "YourPassword" -Force -AsPlainText
Export-PfxCertificate -Cert $cert -FilePath "C:\certs\rust_teams.pfx" -Password $password

Write-Host "Certificate exported to: C:\certs\rust_teams.pfx"
Write-Host "Thumbprint: $($cert.Thumbprint)"
```

## GitHub Secrets Setup

### Add Secrets to Repository
1. Go to GitHub Repository → Settings → Secrets and variables → Actions
2. Add these secrets:

| Secret Name | Value |
|-------------|-------|
| `CODE_SIGNING_CERT_BASE64` | Base64 encoded .pfx file |
| `CODE_SIGNING_CERT_PASSWORD` | Certificate password |

### Encode Certificate to Base64
```powershell
# Encode .pfx to base64
$certBytes = [System.IO.File]::ReadAllBytes("C:\certs\rust_teams.pfx")
$base64 = [Convert]::ToBase64String($certBytes)
$base64 | Set-Clipboard
Write-Host "Base64 copied to clipboard"
```

## How It Works

1. GitHub Actions downloads the certificate from secrets
2. Decodes it from base64 to .pfx file
3. Uses `signtool.exe` to sign the executable
4. Signs with timestamp server for long-term validity
5. Cleans up the certificate file

## Benefits

- ✅ No more "Keep/Delete" warnings in Chrome/Edge
- ✅ Windows SmartScreen trusts the app faster
- ✅ Users see verified publisher name
- ✅ Professional appearance

## Troubleshooting

### "signtool.exe not found"
- Install Windows SDK: https://developer.microsoft.com/en-us/windows/downloads/windows-sdk/

### "Certificate not valid"
- Check certificate hasn't expired
- Verify password is correct
- Ensure certificate is for code signing

### "Timestamp server error"
- Try different timestamp URL: `http://timestamp.sectigo.com`
- Check internet connectivity

## Cost Comparison

| Option | Cost | Validation | Trust Level |
|--------|------|------------|-------------|
| Self-Signed | Free | None | Low (shows warning) |
| Sectigo | ~$50/year | Email | High |
| DigiCert | ~$100/year | Organization | Very High |
| EV Certificate | ~$300/year | Extended | Highest |

## Recommendation

For open-source projects:
1. **Start with self-signed** for testing
2. **Buy Sectigo** (~$50/year) for production
3. **Consider EV** for enterprise customers

---

*Last updated: May 2026*
