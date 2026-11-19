param(
    [Parameter(Mandatory = $true)]
    [string]$Version,
    [switch]$SkipBuild,
    [switch]$NoTag,
    [switch]$PublishGitHub
)

$ErrorActionPreference = "Stop"
$RepoRoot = Split-Path -Parent $PSScriptRoot
$DeckRoot = Join-Path $RepoRoot "deckstudio"
$BinDir = Join-Path $DeckRoot "bin"
$RelDir = Join-Path $RepoRoot ("releases\v" + $Version)
$NotesFile = Join-Path $RelDir "RELEASE_NOTES.md"
$ZipPath = Join-Path $RelDir ("DeckStudio-v" + $Version + "-win-x64.zip")
$SumFile = Join-Path $RelDir "SHA256SUMS.txt"

$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"

Write-Host "==> [1/5] Executando cargo test --workspace..."
Push-Location $DeckRoot
try {
    cargo test --workspace
    if ($LASTEXITCODE -ne 0) {
        throw "cargo test --workspace falhou com código $LASTEXITCODE"
    }
} finally {
    Pop-Location
}

if (-not $SkipBuild) {
    Write-Host "==> [2/5] Compilando deck-engine (release) e deck-studio (Tauri v2)..."
    $wasRunning = [bool](Get-Process deck-engine -ErrorAction SilentlyContinue)
    if ($wasRunning) {
        Get-Process deck-engine -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
        Start-Sleep -Seconds 1
    }

    Push-Location $DeckRoot
    try {
        cargo build --release -p deck-engine
        if ($LASTEXITCODE -ne 0) {
            throw "cargo build --release -p deck-engine falhou ($LASTEXITCODE)"
        }
    } finally {
        Pop-Location
    }

    $StudioDir = Join-Path $DeckRoot "apps\studio"
    Push-Location $StudioDir
    try {
        npm run tauri build
        if ($LASTEXITCODE -ne 0) {
            throw "npm run tauri build falhou ($LASTEXITCODE)"
        }
    } finally {
        Pop-Location
    }

    New-Item -ItemType Directory -Force -Path $BinDir | Out-Null
    Copy-Item -Force (Join-Path $DeckRoot "target\release\deck-engine.exe") (Join-Path $BinDir "deck-engine.exe")
    Copy-Item -Force (Join-Path $StudioDir "src-tauri\target\release\deck-studio.exe") (Join-Path $BinDir "deck-studio.exe")

    if ($wasRunning) {
        Write-Host "==> Reiniciando deck-engine.exe na sessão interativa..."
        $engineExe = Join-Path $BinDir "deck-engine.exe"
        $null = Invoke-CimMethod -ClassName Win32_Process -MethodName Create -Arguments @{ CommandLine = "`"$engineExe`"" }
    }
} else {
    Write-Host "==> [2/5] Build ignorado (-SkipBuild); usando binários existentes em deckstudio\bin\."
}

$EngineBin = Join-Path $BinDir "deck-engine.exe"
$StudioBin = Join-Path $BinDir "deck-studio.exe"
if (-not (Test-Path $EngineBin) -or -not (Test-Path $StudioBin)) {
    throw "Binários ausentes em $BinDir"
}

Write-Host "==> [3/5] Empacotando Release v$Version em $RelDir..."
New-Item -ItemType Directory -Force -Path $RelDir | Out-Null
if (-not (Test-Path $NotesFile)) {
    @"
# DeckStudio v$Version

Release empacotado automaticamente via ``scripts/release.ps1``.
Consulte [CHANGELOG.md](../../CHANGELOG.md) para detalhes completos.
"@ | Set-Content -Path $NotesFile -Encoding UTF8
}

$StageDir = Join-Path $env:TEMP ("DeckStudio_stage_v" + $Version)
Remove-Item -Recurse -Force $StageDir -ErrorAction SilentlyContinue
New-Item -ItemType Directory -Force -Path $StageDir | Out-Null

Copy-Item $EngineBin (Join-Path $StageDir "deck-engine.exe")
Copy-Item $StudioBin (Join-Path $StageDir "deck-studio.exe")
Copy-Item $NotesFile (Join-Path $StageDir "RELEASE_NOTES.md")
foreach ($doc in @("LICENSE", "CHANGELOG.md", "README.md")) {
    $p = Join-Path $RepoRoot $doc
    if (Test-Path $p) { Copy-Item $p (Join-Path $StageDir $doc) }
}
$CompatDoc = Join-Path $RepoRoot "docs\COMPATIBILITY.md"
if (Test-Path $CompatDoc) {
    Copy-Item $CompatDoc (Join-Path $StageDir "COMPATIBILITY.md")
}
$RollbackReg = Join-Path $RepoRoot "rollback\restore_stream_dock_ajazz.reg"
if (Test-Path $RollbackReg) {
    Copy-Item $RollbackReg (Join-Path $StageDir "restore_stream_dock_ajazz.reg")
}

if (Test-Path $ZipPath) { Remove-Item -Force $ZipPath }
Compress-Archive -Path (Join-Path $StageDir "*") -DestinationPath $ZipPath -Force
Remove-Item -Recurse -Force $StageDir -ErrorAction SilentlyContinue

$hEng = (Get-FileHash -Algorithm SHA256 $EngineBin).Hash.ToLowerInvariant()
$hStu = (Get-FileHash -Algorithm SHA256 $StudioBin).Hash.ToLowerInvariant()
$hZip = (Get-FileHash -Algorithm SHA256 $ZipPath).Hash.ToLowerInvariant()
@(
    "$hEng *deck-engine.exe"
    "$hStu *deck-studio.exe"
    "$hZip *$(Split-Path -Leaf $ZipPath)"
) | Set-Content -Path $SumFile -Encoding ASCII

Write-Host "==> SHA256SUMS gerado em $SumFile"

if (-not $NoTag) {
    Write-Host "==> [4/5] Verificando tag Git v$Version..."
    Push-Location $RepoRoot
    try {
        $existingTag = git tag -l "v$Version" 2>$null
        if (-not $existingTag) {
            git tag -a "v$Version" -m "Release v$Version"
            if ($LASTEXITCODE -ne 0) { throw "Falha ao criar tag Git v$Version" }
        }
    } finally {
        Pop-Location
    }
}

if ($PublishGitHub) {
    Write-Host "==> [5/5] Enviando branch/tags e publicando Release no GitHub..."
    Push-Location $RepoRoot
    try {
        $branch = (git rev-parse --abbrev-ref HEAD).Trim()
        git push origin $branch --tags
        if ($LASTEXITCODE -ne 0) { throw "Falha no git push" }

        $existsRel = $false
        $null = gh release view "v$Version" 2>$null
        if ($LASTEXITCODE -eq 0) { $existsRel = $true }

        if ($existsRel) {
            gh release upload "v$Version" $ZipPath $SumFile --clobber
            gh release edit "v$Version" --title "DeckStudio v$Version" --notes-file $NotesFile
        } else {
            gh release create "v$Version" $ZipPath $SumFile --title "DeckStudio v$Version" --notes-file $NotesFile
        }
        if ($LASTEXITCODE -ne 0) { throw "Falha ao publicar GitHub Release v$Version" }
    } finally {
        Pop-Location
    }
}

Write-Host "==> Release v$Version concluída com sucesso!"
