# Diagramas Archify & Matriz de Análise de Impacto — DeckStudio

Este diretório reúne os **10 tipos de diagramas Archify** do projeto **DeckStudio**, gerados e validados com `--quality showcase --browser-check` sobre a revisão `ba55fa104da70fb050df0ce3a69913017efdf260` (release `v0.2.0`).

> [!NOTE]
> **Notas de Transparência & Evidência**
> - **Idioma dos diagramas (`pt-BR`):** Todo o conteúdo dos 10 diagramas está em português do Brasil (`locale: "pt-BR"`). Como o visualizador padrão do Archify ainda não possui catálogo embutido para `pt-BR`, os controles fixos do Viewer (botões de zoom/tema/exportação) e o atributo `<html lang>` exibem fallback em inglês (aviso não bloqueante `i18n/locale-fallback`).
> - **Evidência do `waterfall` vs. demais diagramas:** O diagrama [`waterfall`](./waterfall/deckstudio-waterfall.html) usa `meta.evidence: "illustrative"` (orçamento ilustrativo de latência do ciclo de pressão e renderização HID). O diagrama [`timeline`](./timeline/deckstudio-timeline.html) (`meta.evidence: "observed"`) e os demais diagramas arquiteturais e estruturais estão ancorados diretamente em arquivos, símbolos e intervalos de linha verificados no repositório no commit `ba55fa104da70fb050df0ce3a69913017efdf260`.

---

## Catálogo Completo (10/10 Diagramas Validados)

| # | Tipo | Arquivo HTML Interativo | Propósito Principal | Evidência |
|---:|------|-------------------------|---------------------|-----------|
| 1 | Architecture | [`architecture/deckstudio-architecture.html`](./architecture/deckstudio-architecture.html) | Topologia completa Opção 2 (`deck-engine` 24/7 vs. `deck-studio` sob demanda), subsistemas e hardware HID | Repositório (`ba55fa10`) |
| 2 | Workflow | [`workflow/deckstudio-workflow.html`](./workflow/deckstudio-workflow.html) | Pipeline do ciclo de vida do clique físico, fallback de telemetria e renderização JPEG 96 px | Repositório (`ba55fa10`) |
| 3 | Sequence | [`sequence/deckstudio-sequence.html`](./sequence/deckstudio-sequence.html) | Interação passo a passo: botão físico / API `POST /api/input/press/:pos` → OBS WebSocket v5 → LCD | Repositório (`ba55fa10`) |
| 4 | Dataflow | [`dataflow/deckstudio-dataflow.html`](./dataflow/deckstudio-dataflow.html) | Fluxo de dados entre sensores (`AIDA64`, `LibreHWM`, `HWiNFO64`, `Win32+NVML`), cache de renderização e pacotes HID 512 B | Repositório (`ba55fa10`) |
| 5 | Lifecycle | [`lifecycle/deckstudio-lifecycle.html`](./lifecycle/deckstudio-lifecycle.html) | Máquina de estados dos processos (`deck-engine`, `deck-studio`/WebView2), conexão HID e estados de gravação OBS | Repositório (`ba55fa10`) |
| 6 | ERD | [`erd/deckstudio-erd.html`](./erd/deckstudio-erd.html) | Modelo relacional lógico de `DeckConfig`, `Profile`, `TileConfig`, `TileAction`, `TelemetrySource` e estado em runtime | Repositório (`ba55fa10`) |
| 7 | Class | [`class/deckstudio-class.html`](./class/deckstudio-class.html) | Estruturas Rust (`AppState`, `SensorHub`, `AkpDevice`, `ObsController`, `DeckConfig`) e contratos do frontend | Repositório (`ba55fa10`) |
| 8 | Tree | [`tree/deckstudio-tree.html`](./tree/deckstudio-tree.html) | Hierarquia real de crates, diretórios (`deck-core`, `deck-engine`, `apps/studio`, `design-system/`, `docs/`, `scripts/`) | Repositório (`ba55fa10`) |
| 9 | Timeline | [`timeline/deckstudio-timeline.html`](./timeline/deckstudio-timeline.html) | Evolução cronológica observada: Fase 0 (`QtWebEngine`), protótipo Python, Opção 2 Rust, otimização 19 MB e releases | Observada (`ba55fa10`) |
| 10 | Waterfall | [`waterfall/deckstudio-waterfall.html`](./waterfall/deckstudio-waterfall.html) | Quebra temporal do ciclo de execução (leitura HID, WebSocket OBS, `tiny-skia` + JPEG 512 B, polling assíncrono) | **Ilustrativa** (`illustrative`) |

---

## Matriz de Análise de Impacto (Antes de Modificar)

Antes de alterar qualquer arquivo ou módulo, consulte os diagramas indicados abaixo para avaliar efeitos colaterais em threads, protocolo HID, memória/rede, contratos IPC ou identidade visual. Após mudanças estruturais, atualize os `candidate.json` correspondentes e revalide com `archify finalize`.

| Módulo / Arquivo | Escopo / Responsabilidade | Diagramas a Consultar Antes da Modificação | Principais Riscos a Avaliar |
|------------------|---------------------------|--------------------------------------------|------------------------------|
| `deckstudio/crates/deck-engine/src/hid_proto.rs` | Frame HID de 512 B, protocolo AKP153E (`0300:1010`), mapeamento `6×3`, sanitização JPEG ≤ 96 px / ≤ 10 KiB | `architecture`, `dataflow`, `sequence`, `waterfall`, `class` | Compatibilidade com modelos OEM Mirabox (`docs/COMPATIBILITY.md`), overflow de pacotes USB e corrupção no LCD |
| `deckstudio/crates/deck-engine/src/device.rs` | Thread nativa OS `akp-reader`, timeout ≥ 50 ms, reconexão USB, escrita serializada e `catch_unwind` | `lifecycle`, `sequence`, `workflow`, `waterfall`, `architecture` | Pânico em thread sem runtime Tokio, queda de dispositivo USB ou contenção/latência de rede UDP na NIC |
| `deckstudio/crates/deck-engine/src/actions.rs` | Despacho de ações (`ToggleObsRecord`, `SetObsScene`, `MediaKey`, `RunCommand`, `DiscordAction`, `SwitchPage`) | `workflow`, `sequence`, `class`, `lifecycle` | Perda do `tokio::runtime::Handle` em chamadas vindas de thread OS, estados visuais inconsistentes nos tiles |
| `deckstudio/crates/deck-engine/src/state.rs` | Estado global `AppState`, `dispatch_button_press`, sync de tiles, debounce/cache de renderização | `architecture`, `sequence`, `dataflow`, `class`, `lifecycle` | Condições de corrida (`RwLock`/`Mutex`), re-render desnecessário de teclas (aumento de CPU/RAM) |
| `deckstudio/crates/deck-engine/src/sensors/*` (`mod.rs`, `aida64.rs`, `librehwm.rs`, `hwinfo.rs`, `native_win.rs`) | `SensorHub`, prioridade/fallback automático entre 4 fontes, backoff de 30s, filtro WFP/QoS e standby NVML | `dataflow`, `workflow`, `architecture`, `class`, `waterfall` | Explosão de WorkingSet/Private RAM, falhas em cadeia no fallback, leitura redundante da `nvml.dll` ou bloqueios de rede loopback |
| `deckstudio/crates/deck-engine/src/obs.rs` | Cliente OBS WebSocket v5 (`tokio-tungstenite`), handshake/auth SHA-256, eventos de gravação/cena/áudio | `sequence`, `lifecycle`, `workflow`, `class` | Loop de reconexão em alta frequência, desincronização entre ícone vermelho (`Gravar`) e estado real do OBS |
| `deckstudio/crates/deck-engine/src/weather.rs` e `smart_profile.rs` | Telemetria de clima e troca automática de perfil por janela ativa do Windows | `workflow`, `lifecycle`, `dataflow`, `erd` | Polling excessivo de Win32 `GetForegroundWindow` ou timeouts HTTP impactando o loop de atualização |
| `deckstudio/crates/deck-engine/src/render.rs` | Rasterização `tiny-skia` (`96×96`), tipografia e geração de JPEGs limpos sem EXIF/ICC | `dataflow`, `waterfall`, `architecture`, `class` | Aumento de alocação de memória ou produção de JPEGs > 10 KiB recusados pelo firmware do deck |
| `deckstudio/crates/deck-engine/src/ipc.rs` e `deckstudio/crates/deck-core/src/ipc.rs` | Servidor HTTP `axum` (`127.0.0.1:19876`), DTOs JSON, `POST /api/input/press/:pos` e endpoints de saúde | `architecture`, `sequence`, `class`, `erd`, `dataflow` | Quebra de contrato com `apps/studio/src/api.ts`, exposição indevida fora de `127.0.0.1` |
| `deckstudio/crates/deck-core/src/config.rs` | Esquema `DeckConfig`, perfis, páginas, 18 posições, migração/importação de perfis | `erd`, `class`, `architecture`, `tree` | Perda de retrocompatibilidade de `config.json` do usuário entre versões (`v0.1.0` → `v0.2.0+`) |
| `deckstudio/apps/studio/src/design-system/*`, `styles.css`, `App.tsx` | Tokens semânticos (`--ds-*`), componentes reutilizáveis, 6 abas (`Editor`, `Temas`, `Perfis`, `Fontes`, `Saúde`, `Sobre`), Shell Tauri | `architecture`, `tree`, `class`, `lifecycle` | Violação de tokens do Design System, vazamento de processos `msedgewebview2` ao fechar janela ou regressão de branding |
| `scripts/release.ps1`, `CHANGELOG.md`, `docs/EVOLUTION.md`, `docs/COMPATIBILITY.md` | Empacotamento ZIP + `SHA256SUMS.txt`, tags/releases GitHub e documentação viva | `tree`, `timeline` | Desalinhamento de versão (`Cargo.toml` / `tauri.conf.json` / tag), quebra do fluxo de release versionada |
