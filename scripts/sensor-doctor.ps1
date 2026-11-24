<#
.SYNOPSIS
    DeckStudio - Diagnóstico e Guia de Sensores (AIDA64, LibreHardwareMonitor, HWiNFO64, NVIDIA NVML)
.DESCRIPTION
    Verifica quais provedores de telemetria de CPU, GPU, RAM e FPS estão ativos no Windows
    e exibe os IDs exatos detectados + instruções detalhadas de configuração.
#>
param(
    [string]$LibreUrl = "http://127.0.0.1:8085/data.json",
    [switch]$PauseAtEnd
)

$ErrorActionPreference = "SilentlyContinue"
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "  DeckStudio - Sensor Doctor (Diagnóstico de Telemetria)" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host ""

Add-Type -TypeDefinition @"
using System;
using System.IO;
using System.IO.MemoryMappedFiles;
using System.Text;

public static class ShmReader {
    public static string ReadAida64() {
        try {
            using (var mmf = MemoryMappedFile.OpenExisting("AIDA64_SensorValues", MemoryMappedFileRights.Read))
            using (var stream = mmf.CreateViewStream(0, 0, MemoryMappedFileAccess.Read)) {
                byte[] buf = new byte[16384];
                int read = stream.Read(buf, 0, buf.Length);
                int end = Array.IndexOf(buf, (byte)0, 0, read);
                if (end < 0) end = read;
                return Encoding.Default.GetString(buf, 0, end);
            }
        } catch {
            return null;
        }
    }

    public static bool TestHWiNFO() {
        try {
            using (var mmf = MemoryMappedFile.OpenExisting(@"Global\HWiNFO_SENS_SM2", MemoryMappedFileRights.Read)) {
                return mmf != null;
            }
        } catch {
            return false;
        }
    }
}
"@

# 1. AIDA64 Shared Memory
Write-Host "[1] AIDA64 Shared Memory (AIDA64_SensorValues)" -ForegroundColor Yellow
$aidaProc = Get-Process -Name "aida64*" -ErrorAction SilentlyContinue
$aidaXml = [ShmReader]::ReadAida64()

if (-not [string]::IsNullOrWhiteSpace($aidaXml)) {
    try {
        $settings = New-Object System.Xml.XmlReaderSettings
        $settings.ConformanceLevel = [System.Xml.ConformanceLevel]::Fragment
        $settings.DtdProcessing = [System.Xml.DtdProcessing]::Prohibit
        $settings.XmlResolver = $null
        $settings.MaxCharactersInDocument = 131072
        $sr = New-Object System.IO.StringReader $aidaXml
        $xr = [System.Xml.XmlReader]::Create($sr, $settings)
        $items = @()
        $curKind = ""; $curId = ""; $curLbl = ""; $curVal = ""; $curElem = ""
        while ($xr.Read()) {
            if ($xr.NodeType -eq [System.Xml.XmlNodeType]::Element) {
                if ($xr.Depth -eq 0) { $curKind = $xr.Name; $curId = ""; $curLbl = ""; $curVal = "" }
                else { $curElem = $xr.Name }
            } elseif ($xr.NodeType -eq [System.Xml.XmlNodeType]::Text) {
                if ($curElem -eq "id") { $curId = $xr.Value }
                elseif ($curElem -eq "label") { $curLbl = $xr.Value }
                elseif ($curElem -eq "value") { $curVal = $xr.Value }
            } elseif ($xr.NodeType -eq [System.Xml.XmlNodeType]::EndElement -and $xr.Depth -eq 0) {
                if ($curId -and $curKind -in @("temp", "sys", "fan", "pwr", "volt", "curr")) {
                    $items += [PSCustomObject]@{ Tipo = $curKind; ID = $curId; Label = $curLbl; Valor = $curVal }
                }
            }
        }
        $xr.Close()
        Write-Host "  STATUS: ATIVO ($($items.Count) sensores detectados)" -ForegroundColor Green
        $highlights = $items | Where-Object { $_.ID -in @("TCPU", "SCPUUTI", "PCPUPKG", "SCPUCLK", "TGPU1DIO", "SGPU1UTI", "PGPU1", "DGPU1", "SMEMUTI", "SRTSSFPS") }
        if ($highlights) {
            Write-Host "  Principais sensores mapeados pelo DeckStudio:" -ForegroundColor Gray
            $highlights | Format-Table -AutoSize | Out-String | ForEach-Object { Write-Host $_ }
        }
    } catch {
        Write-Host "  STATUS: Memória encontrada, mas XML incompleto: $_" -ForegroundColor DarkYellow
    }
} else {
    if ($aidaProc) {
        Write-Host "  STATUS: AIDA64 está aberto, mas a Memória Compartilhada está DESLIGADA." -ForegroundColor Red
    } else {
        Write-Host "  STATUS: AIDA64 não está em execução ou SHM desligada." -ForegroundColor DarkGray
    }
    Write-Host "  Como configurar no AIDA64:" -ForegroundColor White
    Write-Host "    1. Abra o AIDA64 Extreme / Engineer"
    Write-Host "    2. Vá em: Arquivo -> Preferências -> Aplicativos Externos"
    Write-Host "    3. Marque 'Ativar memória compartilhada' (AIDA64_SensorValues)"
    Write-Host "    4. Na lista abaixo, marque os itens de CPU, GPU, Memória e RTSS FPS"
    Write-Host "    5. Clique em Aplicar / OK"
    Write-Host ""
}

# 2. LibreHardwareMonitor HTTP
Write-Host "[2] LibreHardwareMonitor Web Server ($LibreUrl)" -ForegroundColor Yellow
$lhmProc = Get-Process -Name "LibreHardwareMonitor*" -ErrorAction SilentlyContinue
if ($LibreUrl -notmatch '^http://(127\.0\.0\.1|localhost|\[::1\]):\d+/') {
    Write-Host "  STATUS: BLOQUEADO (-LibreUrl deve apontar para loopback)" -ForegroundColor Red
} else {
try {
    $resp = Invoke-RestMethod -Uri $LibreUrl -TimeoutSec 2 -MaximumRedirection 0
    Write-Host "  STATUS: ATIVO (Servidor Web respondendo na porta 8085)" -ForegroundColor Green
} catch {
    if ($lhmProc) {
        Write-Host "  STATUS: LibreHardwareMonitor está aberto, mas o Servidor Web está DESLIGADO." -ForegroundColor Red
    } else {
        Write-Host "  STATUS: Offline / não encontrado em $LibreUrl" -ForegroundColor DarkGray
    }
    Write-Host "  Como configurar no LibreHardwareMonitor (Gratuito):" -ForegroundColor White
    Write-Host "    1. Execute LibreHardwareMonitor.exe como Administrador"
    Write-Host "    2. Menu Options -> Remote Web Server -> Run (porta 8085)"
    Write-Host ""
}
}

# 3. HWiNFO64 Shared Memory
Write-Host "[3] HWiNFO64 Shared Memory (Global\HWiNFO_SENS_SM2)" -ForegroundColor Yellow
if ([ShmReader]::TestHWiNFO()) {
    Write-Host "  STATUS: ATIVO (Shared Memory disponível)" -ForegroundColor Green
} else {
    Write-Host "  STATUS: Offline ou Shared Memory desativada." -ForegroundColor DarkGray
    Write-Host "  Como configurar no HWiNFO64:" -ForegroundColor White
    Write-Host "    1. Abra marcando 'Sensors-only'"
    Write-Host "    2. Clique na engrenagem (Settings) -> marque 'Shared Memory Support' -> OK"
    Write-Host ""
}

# 4. Windows Nativo + NVIDIA NVML
Write-Host "[4] Windows Nativo + NVIDIA NVML (Sem programas externos)" -ForegroundColor Yellow
$nvmlPath = Join-Path $env:SystemRoot "System32\nvml.dll"
$nvmlAlt  = Join-Path $env:ProgramFiles "NVIDIA Corporation\NVSMI\nvml.dll"
if ((Test-Path $nvmlPath) -or (Test-Path $nvmlAlt)) {
    Write-Host "  STATUS: ATIVO (Win32 CPU/RAM/Rede + nvml.dll NVIDIA disponível)" -ForegroundColor Green
} else {
    Write-Host "  STATUS: Win32 CPU/RAM/Rede ativo (nvml.dll não encontrada)" -ForegroundColor DarkYellow
}

Write-Host ""
Write-Host "Dica: Se o AIDA64 ou LibreHardwareMonitor estiverem fechados, o DeckStudio" -ForegroundColor Cyan
Write-Host "      faz fallback automático para o próximo provedor da lista!" -ForegroundColor Cyan

if ($PauseAtEnd) {
    Write-Host ""
    Read-Host "Pressione ENTER para fechar"
}
