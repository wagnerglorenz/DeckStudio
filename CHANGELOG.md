# Changelog — DeckStudio

Todas as mudanças notáveis do projeto são registradas aqui seguindo Semantic Versioning (`vX.Y.Z`).

## [0.1.0] - 2026-11-19

### Diagnóstico Inicial & Protótipo Python (`2026-11-18`)
- Diagnóstico de rede (`iperf3` 300 Mbps UDP + Realtek `Ethernet 3`): comprovado que o software oficial **Stream Dock AJAZZ** (`Qt5WebEngineCore.dll` + processos `QtWebEngineProcess.exe`, 252,3 MB RAM) causava 4–9% de perda UDP.
- Protótipo `ajazz_monitor` em Python validando o protocolo HID seguro do AJAZZ AKP153E (`0300:1010`, relatórios de saída de **512 bytes**, sanitização JPEG RGB ≤ 96 px e ≤ 10 KiB sem EXIF/ICC, leitor com timeout ≥ 50 ms) alcançando **0,00% de perda UDP** em 3/3 rodadas e `delta = 0` descartes na NIC.

### DeckStudio v0.1.0 (Arquitetura Opção 2: Engine Rust 24/7 + Editor Tauri v2 sob demanda) (`2026-11-19`)
- **Motor Nativo em Rust (`deck-engine.exe`):** daemon 24/7 na bandeja do Windows sem WebView (`hidapi`, `tiny-skia`, `axum`, `tokio-tungstenite`, `tray-icon`), com API IPC local em `127.0.0.1:19876`.
- **Editor Desktop Sob Demanda (`deck-studio.exe`):** interface Tauri v2 + React + TypeScript com 5 telas (`Editor`, `Temas`, `Perfis`, `Fontes`, `Saúde`); ao fechar a janela, encerra 100% dos processos `msedgewebview2` (`373,71 MB -> 0 MB residuais`).
- **4 Fontes de Telemetria com Guias em Português e Fallback Automático:**
  - `AIDA64 Shared Memory` (`AIDA64_SensorValues`, 79 sensores + vigia opcional de 45s).
  - `LibreHardwareMonitor HTTP` (`http://127.0.0.1:8085/data.json`, timeout 300 ms e backoff de 30s quando offline).
  - `HWiNFO64 Shared Memory` (`Global\HWiNFO_SENS_SM2`).
  - `Windows Nativo + NVIDIA NVML` (`GetSystemTimes`, `GlobalMemoryStatusEx`, `GetIfTable` 1x filtrando filtros WFP/QoS e carregamento sob demanda de `nvml.dll` com standby quando outra fonte já provê GPU).
- **Otimização de Memória & Rede:** consumo reduzido de `41,3–43,9 MB WorkingSet` (`25,9–28,2 MB Private`) para **`18,52–20,32 MB WorkingSet` / `5,32–7,15 MB Private`**, atingindo **0% de perda UDP** em 3/3 rodadas de 300 Mbps (`0/206257`, `0/205593`, `0/206289`), TCP `795,79 Mbps` (0 retransmissões) e `delta = 0` em `ReceivedDiscardedPackets` na `Ethernet 3`.
- **Correção Crítica de Disparo de Botões (`akp-reader`) + Simulação Completa:**
  - Corrigido *panic* em thread nativa do OS ao invocar `tokio::spawn` sem `Handle` (`actions.rs` + `state.rs::dispatch_button_press` + proteção `catch_unwind` em `device.rs`).
  - Adicionado endpoint `POST /api/input/press/:pos` para simulação ao vivo em runtime e suíte de **16/16 testes** (`cargo test --workspace`) incluindo servidor mock OBS WebSocket v5 (`monitor` ↔ `obs`, Gravar, Pausar, Som Jogo, 8 cenas e `Discord`).
