# Changelog — DeckStudio

Todas as mudanças notáveis do projeto são registradas aqui seguindo Semantic Versioning (`vX.Y.Z`).

## [0.4.10] - 2026-11-24

### Correções de UI (Seletor de Tema & Resize na Aba Fontes)
- **Seletor de Tema no Topo ([App.tsx](deckstudio/apps/studio/src/App.tsx)):** `onChange` agora atualiza `cfg.active_theme` e `savedCfg` imediatamente ao trocar de tema e exibe os nomes amigáveis (`LLM Dashboard`, `Spark Clássico`, `Cyberpunk`, `Matrix`, `Obsidian Pro`).
- **Layout Responsivo & Resize na Aba Fontes ([styles.css](deckstudio/apps/studio/src/styles.css), [App.tsx](deckstudio/apps/studio/src/App.tsx)):** corrigido esmagamento do título *"4. Windows Nativo + NVIDIA NVML"* com `flex-wrap` e `white-space: normal` em badges longas, `minmax(0, 1fr)` no `.grid-2` (empilhamento automático abaixo de `1120px`) e painel de sensores redimensionável (`resize: vertical`).

## [0.4.9] - 2026-11-24

### Botões & Logos sem Borda Externa (Fim do Recorte nas Lentes)
- **Renderizador OLED ([render.rs](deckstudio/crates/deck-engine/src/render.rs)):** removidas as molduras/bordas externas e filetes de extremidade em `draw_themed_surface` e `draw_bevel_keycap`, eliminando o efeito de borda cortada nas lentes acrílicas dos botões, cards de sensores e logos (como OBS).

## [0.4.8] - 2026-11-24

### Previsão do Tempo por CEP
- **Busca Automática por CEP (`deck-core`, `deck-engine`, `studio`):** adicionado campo `cep` em `WeatherConfig` ([config.rs](deckstudio/crates/deck-core/src/config.rs)) e rota autenticada `POST /api/weather/cep` ([ipc.rs](deckstudio/crates/deck-engine/src/ipc.rs)) com `lookup_brazil_cep` ([weather.rs](deckstudio/crates/deck-engine/src/weather.rs)), consultando BrasilAPI v2 / ViaCEP + Open-Meteo Geocoding para preencher automaticamente cidade, latitude e longitude.
- **Atualização Imediata no Deck:** adicionada notificação `weather_notify` em `spawn_weather_loop` para buscar o clima e redesenhar o tile `5,2` na hora ao salvar configurações ou resolver um CEP.
- **Interface ([App.tsx](deckstudio/apps/studio/src/App.tsx)):** adicionado campo **CEP** + botão **Buscar CEP** na aba **Saúde** → *Configuração de Clima & OBS*.

## [0.4.7] - 2026-11-24

### Polimento Visual (`/unslop` — Hardware Graphite)
- **Tokens & Cores ([tokens.css](deckstudio/apps/studio/src/design-system/tokens.css), [styles.css](deckstudio/apps/studio/src/styles.css)):** rampa grafite neutra (`#0D0E11`–`#232833`) com hairlines sutis, remoção de glows coloridos e gradiente em `.btn.primary`, mantendo o simulador do deck como momento visual principal.
- **Espaçamento, Tipografia & Motion:** ritmo alinhado à escala de `4px`, redução de pesos `700`/`800` para `400`/`500`/`600`, remoção de `uppercase` em rótulos, `tabular-nums` em KPIs e pills, anéis `:focus-visible` e transições com `cubic-bezier(0.2, 0, 0, 1)`.
- **Ícones & Copy ([components.tsx](deckstudio/apps/studio/src/design-system/components.tsx), [App.tsx](deckstudio/apps/studio/src/App.tsx)):** substituição dos emojis de navegação e botões por ícones de linha `UiIcon` (`16px`, traço `1.5`) e remoção de pontos de exclamação nos avisos da interface.

## [0.4.6] - 2026-11-24

### Desacoplamento de Marca & Padronização (`Stream Dock` / `Spark Clássico`)
- **Tema & Renderização (`deck-core`, `deck-engine`, `studio`):** tema e estilo `ajazz` renomeados para `classic_spark` (**Spark Clássico**) em [config.rs](deckstudio/crates/deck-core/src/config.rs), [render.rs](deckstudio/crates/deck-engine/src/render.rs), [main.rs](deckstudio/crates/deck-engine/src/main.rs), [types.ts](deckstudio/apps/studio/src/types.ts) e [App.tsx](deckstudio/apps/studio/src/App.tsx).
- **Importação de Perfil & UI:** rota e UI de importação renomeadas para `/api/import/manifest` (`ImportManifestRequest`, `import_stream_dock_manifest_json`, `"Importar Perfil (Stream Dock / Mirabox)"`), mantendo a tabela de compatibilidade de hardware na aba Sobre.
- **Repositório Raiz:** diretório legado Python renomeado de `ajazz_monitor` para `legacy_monitor` (com `tests/` e `tools/` atualizados).

## [0.4.5] - 2026-11-24

### Qualidade & Confiabilidade (Clippy + TypeScript Zero Warnings)
- **Limpeza Completa de Código (`deck-core`, `deck-engine`):** removidos símbolos mortos (`CREATE_NO_WINDOW`, `OUTPUT_REQUEST_PREFIX`, `CMD_CLEAR_BUTTON`, `is_open`, `clear_button`, `ObsCmd::RefreshCatalog`, `render_obs_logo_tile`, `to_map`, `WeatherSnapshot.city`), eliminados `.into()` redundantes, `format!` desnecessário e casts redundantes.
- **Idiomaticidade & Segurança de Divisão:** aplicados `sort_by_key`, `div_ceil`, `checked_div`, `is_multiple_of`, `!(3..29).contains(...)`, `&Path` e `next_back()`, zerando todos os avisos em `cargo clippy --workspace --all-targets -- -D warnings`, `deck-studio` (`src-tauri`) e `tsc --noEmit`.

## [0.4.4] - 2026-11-24

### Correções Finais Run-3 (Zero Pendências de Código)
- **Prova Mútua `/api/ping` (`RUN3-001`):** `deck-engine` expõe `GET /api/ping?c=<nonce>` retornando `SHA-256(ipc_token:nonce)` em [ipc.rs](deckstudio/crates/deck-engine/src/ipc.rs) e `deck-studio` verifica a prova antes de enviar o Bearer token ([src-tauri/src/main.rs](deckstudio/apps/studio/src-tauri/src/main.rs)).
- **SSRF DNS + IPv4 Não-Canônico (`SEC-R3-01`):** `actions.rs` normaliza URLs via `reqwest::Url`, bloqueia `0.0.0.0/8`, octal/encurtado e resolve DNS com `tokio::net::lookup_host` + `ClientBuilder::resolve`.
- **`PUT /api/config`, `AIDA64` UTF-8, `OBS`, `UNC` & `sensor-doctor.ps1` (`SEC-R3-02`..`05`, `RUN3-002`):** `put_config` limpa `obs.password` se `obs.host`/`port`/`password_env` mudarem; `parse_aida_xml` alinha `search_end` à fronteira UTF-8; `obs.rs` bloqueia `\` e aplica timeout de 5s no `Hello`; `spawn_program` bloqueia UNC antes de `Command::new`; `sensor-doctor.ps1` usa `-MaximumRedirection 0`.

## [0.4.3] - 2026-11-24

### Defesa em Profundidade Final (Run-2 Completa)
- **`SetDefaultDllDirectories` & Liveness Autenticada:** `deck-engine` ativa `LOAD_LIBRARY_SEARCH_DEFAULT_DIRS` no boot ([main.rs](deckstudio/crates/deck-engine/src/main.rs)) e `deck-studio` valida `GET /api/health` com o token Bearer em `is_engine_running()` ([src-tauri/src/main.rs](deckstudio/apps/studio/src-tauri/src/main.rs)).
- **Updater Pinado ao Check do Servidor:** `POST /api/system/update/install` exige correspondência exata com o `last_update_check` armazenado em `EngineShared` ([ipc.rs](deckstudio/crates/deck-engine/src/ipc.rs)).
- **OBS, Long Press, SensorHub & Diagnóstico:** validação de `obs.host` + timeout de 5s em `connect_async` ([obs.rs](deckstudio/crates/deck-engine/src/obs.rs)), limpeza de `press_times` em troca de perfil ([state.rs](deckstudio/crates/deck-engine/src/state.rs)), `Policy::none()` obrigatório no `SensorHub` ([sensors/mod.rs](deckstudio/crates/deck-engine/src/sensors/mod.rs)) e `XmlReader` com `DtdProcessing::Prohibit` + validação loopback em [sensor-doctor.ps1](scripts/sensor-doctor.ps1).

## [0.4.2] - 2026-11-24

### Hardening Run-2 (Correções de Borda & Defesa em Profundidade)
- **LibreHWM & AIDA64 & Win32 (`librehwm.rs`, `aida64.rs`, `native_win.rs`):** corrigido pânico de fronteira UTF-8 (`chars().take(256)`), streaming com teto de 2 MiB (contra `Transfer-Encoding: chunked`), `!val.is_finite()`, limite `O(N)` no parser XML AIDA64, buffer `Vec<u32>` alinhado em `sample_net_octets` e `nvmlShutdown()` em falha parcial.
- **Webhook SSRF, UNC & Hotkeys (`actions.rs`, `ipc.rs`):** bloqueio de `@` (userinfo), `?`/`#`/`\`, IPv4 octal/hex/decimal/CGNAT e IPv6 privado/mapeado em `is_safe_webhook_url`, redação de URLs em `reqwest::Error`, validação pré-`canonicalize` em `/api/backup/restore` e bloqueio de `\\?\UNC\`.
- **Token, Updater, NSIS & WebView2 (`main.rs`, `updater.rs`, `nsis-hooks.nsh`, `tauri.conf.json`, `api.ts`, `App.tsx`):** checagem estrita de `Host` e retorno de `BCryptGenRandom`, limpeza de `obs.password` ao alterar `obs.host`/`port` na importação, bloqueio de `..`/`%2e`/`@` no updater, `"$SYSDIR\taskkill.exe"` no NSIS, `form-action 'none'` no CSP, retry automático em `401` e limite de 512 KiB em upload PNG.

## [0.4.1] - 2026-11-23

### Hardening Completo de Segurança (`/security-audit`)
- **IPC Autenticado + CORS Restrito (`SEC-001`, `SEC-010`):** token Bearer CSPRNG por sessão (`BCryptGenRandom`) gravado em `%APPDATA%\DeckStudio\ipc.token`, middleware Axum de autenticação em tempo constante e validação de `Host` em [ipc.rs](deckstudio/crates/deck-engine/src/ipc.rs); CORS restrito às origens Tauri; comando `get_ipc_token` no host Tauri ([main.rs](deckstudio/apps/studio/src-tauri/src/main.rs)) e envio automático em [api.ts](deckstudio/apps/studio/src/api.ts).
- **Auto-Updater Endurecido (`SEC-002`, `SEC-009`, `SEC-012`):** pinagem de domínios HTTPS oficiais + Redirect Policy customizada, SHA-256 obrigatório (fail-closed), staging exclusivo com `create_new(true)` em `%LOCALAPPDATA%\DeckStudio\updates`, re-validação pós-gravação contra TOCTOU, trava single-flight (`409 Conflict`), execução desanexada (`DETACHED_PROCESS | CREATE_BREAKAWAY_FROM_JOB`) e remoção de `/T` no `taskkill` em [nsis-hooks.nsh](deckstudio/apps/studio/src-tauri/nsis-hooks.nsh).
- **Confinamento de Caminhos & Proteção de Segredos (`SEC-003`, `SEC-006`, `SEC-013`):** bloqueio de caminhos UNC/dispositivo e allowlist de backups em `/api/backup/restore` e `/api/import/ajazz`; redação de `obs.password` nas respostas IPC e nos backups em `Documentos`; auto-restore silencioso restrito a `AppData`.
- **Ações & Sensores (`SEC-004`, `SEC-005`, `SEC-007`, `SEC-008`, `SEC-012`, `SEC-013`):** abertura de URLs/atalhos via Win32 `ShellExecuteW` (removido `cmd /C start`), filtro anti-SSRF e timeout em Webhooks, `Shell=False` no Python legado, carregamento explícito em `System32` para `nvml.dll`/`winmm.dll`/`schtasks.exe`/`reg.exe`/`powershell.exe`, cópia segura de memória compartilhada (AIDA64/HWiNFO) com `is_finite()`, limites de tamanho/profundidade no LibreHWM, CSP restritiva em [tauri.conf.json](deckstudio/apps/studio/src-tauri/tauri.conf.json), sanitização de `custom_image_b64` e sincronização de `src-tauri/Cargo.toml` em [release.ps1](scripts/release.ps1).

## [0.4.0] - 2026-11-23

### Fase 2: Long Press, Mixer WASAPI, Cronômetro/Pomodoro & Backup/Auto-Restauração
- **Backup Automático & Auto-Restauração na Reinstalação:** [config.rs](deckstudio/crates/deck-core/src/config.rs) grava backups redundantes em `%APPDATA%\DeckStudio\backups\config-latest.json` e `%USERPROFILE%\Documents\DeckStudio\backups\config-latest.json` a cada salvamento, e restaura automaticamente de instalações anteriores (`AppData`, `LocalAppData`, `Documentos`, pasta portátil) quando `config.json` não existe.
- **Exportar / Importar & Restaurar na UI:** novos endpoints `/api/backup/export`, `/api/backup/import`, `/api/backup/sources` e `/api/backup/restore` em [ipc.rs](deckstudio/crates/deck-engine/src/ipc.rs) integrados à aba **Perfis** em [App.tsx](deckstudio/apps/studio/src/App.tsx).
- **Segurar Tecla (Long Press >= 500ms):** suporte a `long_press_action` em [state.rs](deckstudio/crates/deck-engine/src/state.rs) permitindo duas ações na mesma tecla (clique curto vs. segurar).
- **Mixer de Áudio por App (WASAPI Nativo):** nova ação `AudioApp` em [actions.rs](deckstudio/crates/deck-engine/src/actions.rs) para mutar ou ajustar volume de processos individuais (`spotify.exe`, `discord.exe`) ou Master do Windows.
- **Cronômetro / Pomodoro:** widget dinâmico `timer` + ação `Timer` (`toggle`, `pomodoro`, `reset`) em [main.rs](deckstudio/crates/deck-engine/src/main.rs) e [state.rs](deckstudio/crates/deck-engine/src/state.rs).

## [0.3.1] - 2026-11-23

### Qualidade Visual dos Botões (Fim da Penumbra), 3 Novos Temas & Key Creator
- **Eliminação da "Penumbra" nas Teclas Físicas:** substituído o gradiente cinza-azulado lavado (`draw_bevel_keycap`) por superfícies OLED de alto contraste (`draw_themed_surface` em [render.rs](deckstudio/crates/deck-engine/src/render.rs)), molduras nítidas na zona segura da lente acrílica, drop-shadow sutil nos textos e qualidade JPEG inicial `95` em [hid_proto.rs](deckstudio/crates/deck-engine/src/hid_proto.rs) (`<= 10 KiB`).
- **3 Novos Temas Nativos:** adicionados `cyberpunk` (*Night City* — HUD chanfrado amarelo/ciano), `matrix` (*Terminal TTY* — scanlines sutis + verde fósforo) e `obsidian_pro` (*Obsidian Studio Pro* — preto OLED + ouro quente) em [config.rs](deckstudio/crates/deck-core/src/config.rs), com merge automático em `config.json` existente e preview ao vivo na aba **Temas** ([App.tsx](deckstudio/apps/studio/src/App.tsx)).
- **Biblioteca Expandida (45+ Ícones & Logos):** adicionados ícones vetoriais para `cursor`, `docker`, `git`, `powershell`, `notion`, `slack`, `teams`, `zoom`, `epic`, `battlenet`, `firefox`, `brave`, `photoshop`, `premiere`, `davinci`, `blender`, `vlc`, `cpu`, `gpu`, `ram`, `fan`, `volume_up`, `volume_down`, `mute_all`, `audio_switch`, `lock_pc`, `screenshot`, `clipboard`, `stopwatch`, `web` e `lightbulb` em [render.rs](deckstudio/crates/deck-engine/src/render.rs) e [AppIcons.tsx](deckstudio/apps/studio/src/design-system/AppIcons.tsx).
- **Mini Key Creator & Copiar/Colar Tecla:** adicionados seletores de cor customizados por tecla (`bg_color`, `border_color`, `text_color`, `icon_color`) e botões **Copiar / Colar** no Editor ([App.tsx](deckstudio/apps/studio/src/App.tsx)).

## [0.3.0] - 2026-11-23

### Instalador Windows, Auto-Boot & Abertura Direta
- **Instalador Windows NSIS (`DeckStudio_0.3.0_x64-setup.exe`):** configurado empacotamento completo em [tauri.conf.json](deckstudio/apps/studio/src-tauri/tauri.conf.json) incluindo `deck-engine.exe`, `scripts/sensor-doctor.ps1`, atalhos no Menu Iniciar/Área de Trabalho e hooks em [nsis-hooks.nsh](deckstudio/apps/studio/src-tauri/nsis-hooks.nsh) para registrar o motor no boot e limpar processos/registro na desinstalação.
- **Ícone Único Inteligente:** ao abrir o `DeckStudio.exe`, [main.rs (Tauri)](deckstudio/apps/studio/src-tauri/src/main.rs) verifica a porta `127.0.0.1:19876` e inicia automaticamente o `deck-engine.exe` em segundo plano caso ainda não esteja em execução.
- **Controle de Boot do Windows na Dash:** adicionados endpoints `/api/system/autostart` em [ipc.rs](deckstudio/crates/deck-engine/src/ipc.rs) e checkboxes na aba **Saúde** para ativar/desativar a inicialização no boot do motor (bandeja + LCD) e da janela do editor (Dash).

### Sensores, Taxa de Atualização & Contraste Visual
- **Script `sensor-doctor.ps1` + Guia Detalhado AIDA64:** criado [sensor-doctor.ps1](scripts/sensor-doctor.ps1) para diagnosticar ao vivo AIDA64 Shared Memory (`AIDA64_SensorValues`), LibreHardwareMonitor, HWiNFO64 e NVIDIA NVML, acionável direto pelo botão **"🩺 Rodar Script Sensor Doctor"** na aba **Fontes**; guia do AIDA64 ampliado em [sensors/mod.rs](deckstudio/crates/deck-engine/src/sensors/mod.rs).
- **Seletor de Atualização do Display:** adicionado seletor na aba **Saúde** (`250 ms`, `500 ms`, `1000 ms`, `2000 ms`) ligado a `device.refresh_ms`.
- **Botões Menos Opacos:** ajustadas as cores de fundo, bordas e texto do estado padrão (`idle`) e `disabled` no renderizador do LCD ([render.rs](deckstudio/crates/deck-engine/src/render.rs)) e tokens visuais da Dash ([tokens.css](deckstudio/apps/studio/src/design-system/tokens.css), [styles.css](deckstudio/apps/studio/src/styles.css)).

## [0.2.1] - 2026-11-19

### Desenvolvimento Colaborativo com IA Local & Integração OBS Avançada
- **Orquestração Antigravity (Planejador) + Qwen-2.5-Coder (Desenvolvedor):** integração de pipeline através do proxy Ollama na porta `11888` (com medição de tokens para dashboard de consumo), gerando código Rust e TypeScript de alto desempenho a **117–123 tokens/segundo** na GPU local.
- **Feedback Visual Dinâmico em Tempo Real no LCD:**
  - O motor Rust ([main.rs](deckstudio/crates/deck-engine/src/main.rs)) atualizou a função `button_visual_state` para avaliar dinamicamente operações OBS (`toggle_source`, `show_source`, `hide_source`, `toggle_record`, `toggle_stream`, `toggle_virtualcam`, `toggle_studio_mode`, `toggle_mute`, `scene`).
  - Botões físicos agora refletem em tempo real no LCD (800x480) o estado das fontes (`VISÍVEL` / `active` com destaque de borda, ou `OCULTO` / `idle`), mutação de microfone (`MUDO` / `ATIVO`) e cenas ativas.
  - Adicionado novo teste unitário de regressão `test_obs_source_dynamic_visual_state` (19/19 testes aprovados no workspace).
- **Inspetor Visual de Fontes OBS no DeckStudio (`ObsSourcesInspector.tsx`):**
  - Desenvolvido novo componente no Design System ([ObsSourcesInspector.tsx](deckstudio/apps/studio/src/design-system/ObsSourcesInspector.tsx)) com filtro de busca em tempo real, badges temáticos categorizados por heurística (📷 `CAM`, 🔊 `ÁUDIO`, 🖥️ `TELA`, 🖼️ `IMAGEM`, 📦 `FONTE`) e vinculação direta da fonte à tecla selecionada com 1-clique.
  - Integrado no editor de ações do Studio ([App.tsx](deckstudio/apps/studio/src/App.tsx)).

## [0.2.0] - 2026-11-19

### Marca, Design System & Experiência do Usuário
- **Identidade Visual Própria:** criada logomarca vetorial oficial do **DeckStudio** ([logo.svg](deckstudio/assets/logo.svg)), ícone multi-resolução Windows ([icon.ico](deckstudio/apps/studio/src-tauri/icons/icon.ico) `16..256 px`) e novo ícone de bandeja pixel-art em [main.rs](deckstudio/crates/deck-engine/src/main.rs).
- **Identificação Limpa (Sem Marca Específica no Título):** removido `AJAZZ AKP153E` do título da janela (`DeckStudio`), título HTML, cabeçalho do editor e tooltip da bandeja (`DeckStudio Engine`).
- **Nova Aba `Sobre` + Compatibilidade:** adicionada tela dedicada em [App.tsx](deckstudio/apps/studio/src/App.tsx) destacando o projeto como **alternativa 100% gratuita (Licença MIT)** e exibindo a tabela de modelos compatíveis verificados e candidatos OEM Mirabox.
- **Design System Estruturado:** criados [tokens.css](deckstudio/apps/studio/src/design-system/tokens.css) (`--ds-*`), [components.tsx](deckstudio/apps/studio/src/design-system/components.tsx) (`DeckLogo`, `Button`, `StatusPill`, `Card`, `KpiCard`, `Field`) e documentação em [docs/DESIGN_SYSTEM.md](docs/DESIGN_SYSTEM.md).

### Compatibilidade, Evolução & Análise de Impacto Archify
- Pesquisa e documentação detalhada de hardware compatível e protocolos similares (`AJAZZ AKP153 / AKP153E / AKP153R`, `Mirabox HSV293S / V3`, `VSDinside N3 / N4`, `AKP03`) em [docs/COMPATIBILITY.md](docs/COMPATIBILITY.md).
- Novo artefato permanente de evolução e benchmarks em [docs/EVOLUTION.md](docs/EVOLUTION.md).
- Suite completa de **10 diagramas Archify validados** (`architecture`, `workflow`, `sequence`, `dataflow`, `lifecycle`, `erd`, `class`, `tree`, `timeline`, `waterfall`) + matriz de análise de impacto em [docs/archify/README.md](docs/archify/README.md).
- Removido aviso de importação não usada (`hid_proto::InputAction`) na build release do `deck-engine`.

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
