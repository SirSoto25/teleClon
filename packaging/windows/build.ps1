# Empaqueta Parlo para Windows x86_64: parlo.exe, zip portable e instalador Inno Setup. ADR-0006.
# Uso (PowerShell, desde cualquier directorio):
#   pwsh packaging/windows/build.ps1 [-SkipBuild] [-SkipInstaller]
# Requisitos: Rust (MSVC, target x86_64-pc-windows-msvc) con Windows SDK (rc.exe, signtool.exe) e
# Inno Setup 6 (ISCC.exe) para el instalador.
# Firma Authenticode OPCIONAL: solo si existen WINDOWS_CERT_PFX_BASE64 y WINDOWS_CERT_PASSWORD
# (certificado .pfx en base64 y su contraseña). Sin ellas el resultado va sin firmar (SmartScreen avisará).
# Salida: dist/Parlo-<versión>-windows-x86_64-portable.zip y dist/Parlo-<versión>-windows-x86_64-setup.exe

param(
    [switch]$SkipBuild,
    [switch]$SkipInstaller
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

$Root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$Target = 'x86_64-pc-windows-msvc'
$Dist = Join-Path $Root 'dist'
$Stage = Join-Path $Dist 'staging\windows'
$Pfx = $null
$SignTool = $null

Push-Location $Root
try {
    $meta = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
    $Version = ($meta.packages | Where-Object { $_.name -eq 'tc-app' }).version
    Write-Host "Parlo $Version ($Target)"

    if (-not $SkipBuild) {
        cargo build --release --locked -p tc-app --target $Target
        if ($LASTEXITCODE -ne 0) { throw 'cargo build falló' }
    }
    $Exe = Join-Path $Root "target\$Target\release\parlo.exe"
    if (-not (Test-Path $Exe)) { throw "no existe $Exe" }

    # --- Firma (opcional) ----------------------------------------------------------------------
    if ($env:WINDOWS_CERT_PFX_BASE64 -and $env:WINDOWS_CERT_PASSWORD) {
        $Pfx = Join-Path ([System.IO.Path]::GetTempPath()) 'parlo-signing.pfx'
        [System.IO.File]::WriteAllBytes($Pfx, [System.Convert]::FromBase64String($env:WINDOWS_CERT_PFX_BASE64))
        $SignTool = Get-ChildItem "${env:ProgramFiles(x86)}\Windows Kits\10\bin\*\x64\signtool.exe" -ErrorAction SilentlyContinue |
            Sort-Object FullName -Descending | Select-Object -First 1 -ExpandProperty FullName
        if (-not $SignTool) { throw 'hay certificado pero no se encontró signtool.exe (Windows SDK)' }
    } else {
        Write-Host 'Sin WINDOWS_CERT_PFX_BASE64/WINDOWS_CERT_PASSWORD: se empaqueta SIN firmar.'
    }
    function Sign-File([string]$Path) {
        if (-not $SignTool) { return }
        $ts = if ($env:WINDOWS_TIMESTAMP_URL) { $env:WINDOWS_TIMESTAMP_URL } else { 'http://timestamp.digicert.com' }
        & $SignTool sign /fd SHA256 /td SHA256 /tr $ts /f $Pfx /p $env:WINDOWS_CERT_PASSWORD /d 'Parlo' $Path
        if ($LASTEXITCODE -ne 0) { throw "signtool falló con $Path" }
    }

    # --- Carpeta de staging ----------------------------------------------------------------------
    if (Test-Path $Stage) { Remove-Item -Recurse -Force $Stage }
    New-Item -ItemType Directory -Force -Path (Join-Path $Stage 'licenses') | Out-Null
    Copy-Item $Exe (Join-Path $Stage 'parlo.exe')
    Sign-File (Join-Path $Stage 'parlo.exe')
    Copy-Item (Join-Path $Root 'README.md') $Stage
    Copy-Item (Join-Path $Root 'assets\fonts\NOTICE.md') (Join-Path $Stage 'licenses\FONTS-NOTICE.md')
    foreach ($f in Get-ChildItem (Join-Path $Root 'assets\fonts') -Directory) {
        Copy-Item (Join-Path $f.FullName 'OFL.txt') (Join-Path $Stage "licenses\OFL-$($f.Name).txt")
    }
    Copy-Item (Join-Path $Root 'assets\icons\NOTICE.md') (Join-Path $Stage 'licenses\ICONS-NOTICE.md')
    Copy-Item (Join-Path $Root 'assets\icons\LICENSE-MIT.txt') (Join-Path $Stage 'licenses\ICONS-LICENSE-MIT.txt')
    Copy-Item (Join-Path $Root 'assets\licenses\OpenH264-BSD-2-Clause.txt') (Join-Path $Stage 'licenses\OpenH264-BSD-2-Clause.txt')

    # --- Zip portable ------------------------------------------------------------------------
    $Zip = Join-Path $Dist "Parlo-$Version-windows-x86_64-portable.zip"
    if (Test-Path $Zip) { Remove-Item -Force $Zip }
    Compress-Archive -Path (Join-Path $Stage '*') -DestinationPath $Zip
    Write-Host "zip: $Zip"

    # --- Instalador ----------------------------------------------------------------------------
    if (-not $SkipInstaller) {
        $Iscc = (Get-Command ISCC.exe -ErrorAction SilentlyContinue).Source
        if (-not $Iscc) {
            $Iscc = @("${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe", "$env:ProgramFiles\Inno Setup 6\ISCC.exe", "$env:LOCALAPPDATA\Programs\Inno Setup 6\ISCC.exe") |
                Where-Object { Test-Path $_ } | Select-Object -First 1
        }
        if (-not $Iscc) { throw 'no se encontró ISCC.exe (Inno Setup 6). Instálalo o usa -SkipInstaller.' }
        & $Iscc "/DAppVersion=$Version" "/DSourceDir=$Stage" "/DOutputDir=$Dist" (Join-Path $PSScriptRoot 'parlo.iss')
        if ($LASTEXITCODE -ne 0) { throw 'ISCC falló' }
        $Setup = Join-Path $Dist "Parlo-$Version-windows-x86_64-setup.exe"
        Sign-File $Setup
        Write-Host "instalador: $Setup"
    }
}
finally {
    if ($Pfx -and (Test-Path $Pfx)) { Remove-Item -Force $Pfx }
    Pop-Location
}
