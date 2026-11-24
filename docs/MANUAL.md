# Manual Completo — DeckStudio

Guia completo de funcionalidades, configuração, arquitetura e segurança do **DeckStudio** (v0.4.10).

---

## 1. Visão Geral & Arquitetura

O DeckStudio é dividido em dois processos independentes para manter o consumo de memória e rede próximo de zero em uso 24/7:

| Componente | Tecnologia | Papel |
|---|---|---|
| **`deck-engine.exe`** | Rust + `hidapi` + `tokio` + `axum` + `tiny-skia` | Roda continuamente na bandeja do Windows: comunica-se via HID (512 bytes) com o hardware, lê sensores, renderiza as telas OLED em JPEG (`<= 10 KiB`), escuta teclas físicas, controla OBS/áudio/macros e expõe a API local `127.0.0.1:19876`. |
| **`DeckStudio.exe`** | Tauri v2 + React 19 + TypeScript | Interface visual de edição e diagnóstico. Ao fechar a janela, 100% dos processos `msedgewebview2` são encerrados enquanto o motor continua ativo na bandeja. |
| **`deck-core`** | Biblioteca Rust compartilhada | Esquema de `AppConfig`, temas, perfis, regras de cor, parser de manifestos e sistema de backup/restauração. |

### Segurança da Comunicação Local (IPC)
- A cada inicialização, `deck-engine` gera um token CSPRNG (`BCryptGenRandom`, 256 bits) gravado em `%APPDATA%\DeckStudio\ipc.token`.
- `DeckStudio.exe` valida a identidade do motor via desafio/prova (`GET /api/ping?c=<nonce>` -> `SHA-256(ipc_token:nonce)`) antes de enviar o token `Bearer`.
- Todas as rotas `/api/*` exigem cabeçalho `Host` loopback (`127.0.0.1:19876`), token `Bearer` validado em tempo constante e CORS restrito às origens Tauri.

---

## 2. Instalação, Inicialização & Auto-Updater

- **Instalador NSIS (`DeckStudio_<versao>_x64-setup.exe`):**
  - Encerra instâncias anteriores de `deck-engine.exe` e `DeckStudio.exe` antes de atualizar.
  - Instala o aplicativo no perfil do usuário (`currentUser`), registra `deck-engine.exe` em `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` e inicia o motor na bandeja.
- **Auto-Updater (aba *Sobre* ou banner no topo):**
  - Consulta a última release oficial no GitHub (`wagnerglorenz/DeckStudio`).
  - Exige HTTPS em domínios oficiais do GitHub, redirecionamento controlado, verificação obrigatória de **SHA-256** (`SHA256SUMS.txt`) e staging exclusivo em `%LOCALAPPDATA%\DeckStudio\updates` antes de executar o instalador.

---

## 3. Editor Visual & Grade Física (Aba **Editor**)

O hardware suportado possui **15 teclas principais (grade `5×3`, chaves `0,0` a `4,2`)** e **3 áreas na faixa lateral (`5,0`, `5,1`, `5,2`)**:

- **Seletor de Perfil & Tema (barra superior):** troca o perfil ativo ou o tema visual ao vivo.
- **Controle de Brilho & Modo Noturno:** ajuste imediato (`0–100%`) e agendamento automático de brilho reduzido por horário.
- **Páginas & Pastas:**
  - Crie, alterne ou exclua páginas (`principal`, `obs`, etc.).
  - Botões do tipo **Pasta (`Folder`)** abrem subpáginas empilhadas, com botão **Voltar (`FolderBack`)** gerado automaticamente.
- **Edição de Tecla (Inspetor abaixo do deck):**
  - Clique em qualquer tecla para editar tipo (`sensor`, `button`), título, subtítulo, estilo do card (`ring` ou barra), cor/tema, ícone (45+ ícones vetoriais ou PNG customizado até 512 KiB), ação principal e ação de **Segurar Tecla (Long Press ≥ 500 ms)**.
  - **Copiar / Colar Tecla:** duplica configurações completas entre posições.
  - **Desfazer / Refazer (`Ctrl+Z` / `Ctrl+Y`):** histórico local de edições antes de salvar.
  - **Mini Criador de Tecla:** assistente rápido para montar teclas prontas (apps populares, cenas OBS, atalhos, áudio ou timer) em poucos cliques.
- **Biblioteca Lateral (direita):** arraste templates prontos (sensores, apps, OBS, mídia, navegação) diretamente para qualquer slot do deck.

---

## 4. Ações Suportadas (Clique Curto & Long Press)

Todas as ações podem ser atribuídas ao **clique curto** e, opcionalmente, uma segunda ação ao **segurar a tecla (`long_press_action`, ≥ 500 ms)**:

| Tipo (`kind`) | O que faz |
|---|---|
| `Hotkey` | Envia combinações de teclas Win32 (`CTRL+SHIFT+M`, `F13`, `ALT+TAB`, etc.). |
| `Program` | Abre executáveis locais (com expansão de variáveis como `%LOCALAPPDATA%`, bloqueio de caminhos UNC). |
| `Url` | Abre links `http://`/`https://` ou protocolos seguros via `ShellExecuteW`. |
| `Media` | Play/Pause, Próxima, Anterior, Aumentar/Diminuir Volume e Mudo global. |
| `Obs` | Controla OBS WebSocket v5: `toggle_record`, `pause_record`, `toggle_stream`, `toggle_vcam`, `toggle_replay`, `save_replay`, `studio_mode`, `transition`, `scene`, `toggle_mute`, `toggle_source`, `set_source`. |
| `AudioApp` | **Mixer WASAPI nativo por aplicativo:** muta (`mute`) ou ajusta volume (`vol_up` / `vol_down`) de processos específicos (`spotify.exe`, `discord.exe`, etc.) ou do `Master` do Windows. |
| `Timer` | **Cronômetro / Pomodoro:** alterna pausa/execução (`toggle`), inicia ciclo Pomodoro 25 min (`pomodoro`) ou zera (`reset`), exibindo contagem regressiva no widget `timer`. |
| `Page` / `Folder` / `FolderBack` | Navegação entre páginas e pastas empilhadas. |
| `Theme` / `Brightness` | Troca o tema visual ou ajusta o brilho do hardware direto pelo deck. |
| `Webhook` | Dispara requisições HTTP `GET`/`POST` com proteção anti-SSRF (bloqueio de IPs privados, loopback, link-local, DNS rebinding). |
| `Soundboard` | Reproduz arquivos `.wav` locais validados via `winmm.dll`. |
| `MultiAction` | Executa sequências ordenadas de ações e pausas (`delay` em ms). |

---

## 5. Temas & Renderização OLED (Aba **Temas**)

- **5 Temas integrados:**
  1. `llm_dash` — **LLM Dashboard (OLED):** fundo preto absoluto, anéis/barras de alto contraste.
  2. `classic_spark` — **Spark Clássico:** gráficos sparkline em degradê magenta com unidades sobrescritas.
  3. `cyberpunk` — **Cyberpunk · Night City:** amarelo elétrico e ciano sobre fundo escuro.
  4. `matrix` — **Matrix · Terminal TTY:** verde fósforo com scanlines sutis.
  5. `obsidian_pro` — **Obsidian Studio Pro:** preto absoluto com acentos dourados.
- **Renderização limpa:** todas as teclas e logos são renderizadas em `tiny-skia` sem bordas externas ou molduras próximas às extremidades, evitando recorte visual pelas lentes acrílicas do deck.

---

## 6. Perfis, Smart Profiles, Backup & Multi-Action (Aba **Perfis**)

- **Smart Profiles (`SetWinEventHook · 0% Polling`):** monitora mudanças de janela ativa no Windows via eventos nativos (zero CPU em idle) e troca automaticamente para o perfil/página configurado para aquele executável (ex.: `obs64.exe` -> página `obs`; `AC2-Win64-Shipping.exe` -> página `monitor`).
- **Backup Automático & Auto-Restauração:**
  - A cada salvamento, grava cópias redundantes em `%APPDATA%\DeckStudio\backups\config-latest.json` e `%USERPROFILE%\Documents\DeckStudio\backups\config-latest.json` (com redação da senha do OBS no backup de Documentos).
  - Na reinstalação (quando `config.json` ainda não existe), restaura automaticamente o último backup privado de `AppData`/`LocalAppData`.
  - Permite também **Exportar (.json)**, **Importar (.json)**, **Buscar Anteriores** e **Importar Perfil (Stream Dock / Mirabox)** a partir de um `manifest.json` compatível (`POST /api/import/manifest`).
- **Construtor de Multi-Action:** monte sequências com múltiplas ações + atrasos em milissegundos, teste ao vivo e grave diretamente na tecla selecionada.

---

## 7. Fontes de Telemetria & Sensores (Aba **Fontes**)

Prioridade em cascata com fallback automático nos aliases (`CPU_TEMP`, `GPU_TEMP`, `CPU_UTIL`, `GPU_UTIL`, `RAM_UTIL`, etc.):

1. **AIDA64 Shared Memory (`AIDA64_SensorValues`):** leitura direta da memória compartilhada + watchdog de 45s.
2. **LibreHardwareMonitor (`http://127.0.0.1:8085/data.json`):** gratuito e open-source, com backoff de 30s quando offline.
3. **HWiNFO64 Shared Memory (`Global\HWiNFO_SENS_SM2`):** leitura direta de sensores (versão gratuita desliga SHM após 12h).
4. **Windows Nativo + NVIDIA NVML:** leitura nativa sem apps externos via Win32 (`GetSystemTimes`, `GlobalMemoryStatusEx`, `GetIfTable` filtrando adaptadores virtuais WFP/QoS) e `nvml.dll` (em standby automático quando outra fonte prioritária já fornece GPU).

- **Explorador de Sensores ao Vivo:** busca em tempo real por ID/nome, filtro por fonte e botão **Mapear (`x,y`)** para vincular qualquer sensor diretamente à tecla selecionada.
- **Sensor Doctor (`scripts/sensor-doctor.ps1`):** diagnóstico guiado em PowerShell para verificar AIDA64, LibreHWM, HWiNFO e NVML.

---

## 8. Saúde, Rede, Clima por CEP & OBS (Aba **Saúde**)

- **KPIs em tempo real:** RAM privada do `deck-engine.exe`, taxa de envio HID (512B com rate-limit e hash diff), descartes da placa de rede, status do OBS WebSocket v5 e temperatura local.
- **Clima por CEP (Card `5,2`):** informe seu **CEP** (`00000-000`) e clique em **Buscar CEP** (`POST /api/weather/cep`). O motor consulta **BrasilAPI v2** (com fallback para **ViaCEP** + **Open-Meteo Geocoding**), preenche automaticamente cidade, latitude e longitude, e atualiza a previsão no deck imediatamente.
- **OBS Studio WebSocket v5:** configuração de host, porta e senha (ou variável de ambiente `OBS_WEBSOCKET_PASSWORD`), com inspetor de cenas e fontes ao vivo no Editor.

---

## 9. Menu da Bandeja (System Tray)

O ícone do `deck-engine.exe` na bandeja do Windows oferece acesso rápido sem abrir a interface:
- **Abrir DeckStudio**
- **Tema:** `LLM Dashboard` / `Spark Clássico`
- **Brilho:** `20%`, `40%`, `60%`, `80%`, `100%`
- **Pausar / Retomar** envio ao hardware
- **Sair**

---

## 10. Desenvolvimento, Testes & Release

```powershell
# Validar tipagem TypeScript
Set-Location deckstudio\apps\studio
npx tsc --noEmit

# Validar lint estrito e testes Rust (25 testes)
Set-Location ..\..
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

# Gerar build completa + instalador NSIS + SHA256SUMS.txt
Set-Location ..
.\scripts\release.ps1 -Version "0.4.10"
```

Para mais detalhes técnicos, consulte:
- [docs/COMPATIBILITY.md](COMPATIBILITY.md) — Compatibilidade de hardware e protocolo HID.
- [docs/DESIGN_SYSTEM.md](DESIGN_SYSTEM.md) — Tokens semânticos (`tokens.css`) e componentes React.
- [docs/EVOLUTION.md](EVOLUTION.md) — Histórico de evolução e benchmarks.
- [docs/archify/README.md](archify/README.md) — Diagramas Archify de arquitetura e impacto.
