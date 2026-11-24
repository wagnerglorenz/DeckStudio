<#
.SYNOPSIS
    DeckStudio - Script de Release Versionada & Publicação Automática
.DESCRIPTION
    Atualiza a versão nos manifestos, executa todos os testes, compila o motor Rust e o instalador NSIS,
    gera SHA256SUMS.txt e opcionalmente publica uma GitHub Release para o Auto-Updater.
.EXAMPLE
    .\scripts\release.ps1 -Version 0.3.1
    .\scripts\release.ps1 -Version 0.3.1 -Publish -Notes "Correções e melhorias"
#>
param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^\d+\.\d+\.\d+$')]
    [string]$Version,
    [string]$Notes = "Atualização do DeckStudio v$Version",
    [switch]$Publish
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$DeckStudioDir = Join-Path $Root "deckstudio"
$StudioDir = Join-Path $DeckStudioDir "apps\studio"

Write-Host "==> [1/5] Sincronizando versão $Version nos manifestos..." -ForegroundColor Cyan

$Utf8NoBom = [System.Text.UTF8Encoding]::new($false)

# Cargo.toml (deck-core, deck-engine & deck-studio)
foreach ($cargo in @(
    (Join-Path $DeckStudioDir "crates\deck-core\Cargo.toml"),
    (Join-Path $DeckStudioDir "crates\deck-engine\Cargo.toml"),
    (Join-Path $StudioDir "src-tauri\Cargo.toml")
)) {
    $raw = [System.IO.File]::ReadAllText($cargo)
    $updated = [regex]::Replace($raw, '(?ms)(\[package\][^\[]*?^version\s*=\s*)"[^"]+"', "`$1`"$Version`"", 1)
    [System.IO.File]::WriteAllText($cargo, $updated, $Utf8NoBom)
}

# package.json
$pkgJson = Join-Path $StudioDir "package.json"
$pkgUpdated = ([System.IO.File]::ReadAllText($pkgJson)) -replace '"version": "[^"]+"', "`"version`": `"$Version`""
[System.IO.File]::WriteAllText($pkgJson, $pkgUpdated, $Utf8NoBom)

# tauri.conf.json
$tauriConf = Join-Path $StudioDir "src-tauri\tauri.conf.json"
$tauriUpdated = ([System.IO.File]::ReadAllText($tauriConf)) -replace '"version": "[^"]+"', "`"version`": `"$Version`""
[System.IO.File]::WriteAllText($tauriConf, $tauriUpdated, $Utf8NoBom)

Write-Host "==> [2/5] Executando suíte de testes Rust (cargo test --workspace)..." -ForegroundColor Cyan
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
Push-Location $DeckStudioDir
cargo test --workspace
if ($LASTEXITCODE -ne 0) { throw "Falha nos testes Rust!" }

Write-Host "==> [3/5] Compilando deck-engine.exe (release)..." -ForegroundColor Cyan
cargo build --release --bin deck-engine
if ($LASTEXITCODE -ne 0) { throw "Falha ao compilar deck-engine!" }
Pop-Location

Write-Host "==> [4/5] Compilando DeckStudio e gerando instalador NSIS..." -ForegroundColor Cyan
Push-Location $StudioDir
npm run tauri -- build
if ($LASTEXITCODE -ne 0) { throw "Falha ao gerar instalador NSIS!" }
Pop-Location

$BundleDir = Join-Path $StudioDir "src-tauri\target\release\bundle\nsis"
$SetupExe = Join-Path $BundleDir "DeckStudio_${Version}_x64-setup.exe"
if (-not (Test-Path $SetupExe)) {
    throw "Instalador não encontrado em $SetupExe"
}

Write-Host "==> [5/5] Gerando SHA256SUMS.txt..." -ForegroundColor Cyan
$Hash = (Get-FileHash -Path $SetupExe -Algorithm SHA256).Hash.ToLower()
$SumsFile = Join-Path $BundleDir "SHA256SUMS.txt"
"$Hash  $(Split-Path -Leaf $SetupExe)" | Set-Content $SumsFile -Encoding ASCII

Write-Host ""
Write-Host "Release v$Version pronta!" -ForegroundColor Green
Write-Host "  Instalador : $SetupExe"
Write-Host "  SHA-256    : $Hash"
Write-Host "  Checksums  : $SumsFile"

if ($Publish) {
    if (-not (Get-Command gh -ErrorAction SilentlyContinue)) {
        throw "GitHub CLI (gh) não encontrado no PATH para publicação automática."
    }
    Write-Host "==> Publicando GitHub Release v$Version..." -ForegroundColor Cyan
    gh release create "v$Version" $SetupExe $SumsFile --title "DeckStudio v$Version" --notes $Notes
}
