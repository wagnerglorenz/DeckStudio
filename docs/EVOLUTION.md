# Evolução do Projeto — DeckStudio

Registro histórico dos marcos técnicos, decisões arquiteturais e medições reais de desempenho desde o diagnóstico inicial do software oficial até o **DeckStudio v0.2.0**.

## Linha do Tempo e Decisões

| Data | Marco / Versão | Problema / Decisão | Evidência Medida |
| :--- | :--- | :--- | :--- |
| `2026-11-18` | **Fase 0 (Diagnóstico Oficial)** | Software oficial `Stream Dock AJAZZ` (`Qt5WebEngineCore.dll` 102 MB + até 8 processos `QtWebEngineProcess.exe`) causava picos de interrupção/DPC e perda de pacotes UDP em jogo/streaming. | **252,3 MB RAM**, `+13k irq/s`, **4–9% de perda UDP** em `iperf3` 300M. |
| `2026-11-18` | **Protótipo Python (`ajazz-monitor`)** | Eng. reversa segura no Windows descobriu que o AKP153E (`0300:1010`) usa **512B + report ID `0x00`** (513B) e calibragem 6×3 (`4,0→1 .. 5,2→18`). Removido WebView, integrado AIDA64 SHM, `GetIfTable` 1x, OBS v5, clima Santa Maria e botão Discord. | **4,2 MB RAM**, **0% CPU**, **0,00% perda UDP** (3/3), `delta = 0` descartes Realtek. |
| `2026-11-19` | **Opção 2 — DeckStudio (`v0.1.0`)** | Migração para **Rust** (`deck-engine.exe` 24/7 na bandeja sem WebView) + **Tauri v2 / React** (`deck-studio.exe` sob demanda com 0 MB WebView2 ao fechar) + 4 fontes de sensores com guias na UI. | **14→16 testes OK**; editor fecha de `373,71 MB` para **`0 MB` residuais**. |
| `2026-11-19` | **Otimização RAM & DPC (`v0.1.0`)** | Detectado que sondagem HTTP a cada 1s na porta `:8085` fechada, `nvml.dll` residente junto ao AIDA64 e re-renderização de 18 teclas estáticas elevavam RAM (`~43 MB`) e causaram 1 rodada UDP com 2,85% de perda. Adicionado backoff 30s no LibreHWM, lazy/standby do NVML e cache de assinatura em `render_all_tiles`. | RAM caiu para **`18,5–20,3 MB WorkingSet` (`5,3–7,15 MB Private`)**; **`0%` perda UDP** em 3/3 e **`delta = 0`** na `Ethernet 3`. |
| `2026-11-19` | **Correção Botões Físicos (`v0.1.0`)** | Thread `akp-reader` (thread OS nativa) entrava em *panic* ao chamar `tokio::spawn` sem `Handle`. Passado `rt.handle()` + `catch_unwind` + mock OBS v5 e `POST /api/input/press/:pos`. | **16/16 testes OK**; todas as teclas `monitor` ↔ `obs` e cenas validadas ao vivo; Release `v0.1.0` publicado no GitHub privado. |
| `2026-11-19` | **DeckStudio `v0.2.0`** | Implantado **Design System** (`tokens.css` + `components.tsx`), criada logomarca SVG/ICO própria, removido `AJAZZ AKP153E` do título/bandeja, adicionada aba **Sobre** + pesquisa de compatibilidade (`docs/COMPATIBILITY.md`), Licença MIT, persistência na memória global do Antigravity e geração completa dos **10 diagramas Archify**. | Verificação completa de builds, UI ao vivo, `v0.2.0` Release e 10 diagramas HTML validados via `browser-check`. |

## Comparativo Consolidado

| Métrica | Stream Dock AJAZZ (Oficial) | DeckStudio (`v0.2.0`) |
| :--- | :--- | :--- |
| **RAM em 2º plano (bandeja)** | `~252,3 MB` (até 8× `QtWebEngineProcess`) | **`~18,5–20,3 MB` WorkingSet (`5,3–7,2 MB` Priv.)** |
| **WebView em idle** | Sempre residente em memória | **`0 MB`** (encerra ao fechar o editor) |
| **Perda UDP (300 Mbps × 8s)** | `4–9%` observada | **`0%` em 3/3 rodadas** (`delta = 0` NIC discards) |
| **Fontes de Sensores** | Limitada / proprietária | **4 fontes + fallback:** AIDA64, LibreHWM, HWiNFO64, Win32+NVML |
| **Licença & Custo** | Proprietário (marca específica) | **100% Gratuito · Licença MIT** |
