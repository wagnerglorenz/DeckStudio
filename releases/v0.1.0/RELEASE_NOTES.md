# DeckStudio v0.1.0 (2026-11-19)

Primeira versão estável da arquitetura **Opção 2** (Motor Rust 24/7 + Editor Tauri v2 sob demanda), construída como alternativa nativa, leve e gratuita ao software oficial **Stream Dock AJAZZ**.

## Destaques da Versão
- **Compatibilidade Verificada:** `AJAZZ AKP153E` (`VID 0x0300`, `PID 0x1010`, protocolo HID 512 bytes, grade 5×3 + faixa lateral 3×1).
- **Consumo Medido:**
  - `deck-engine.exe` em bandeja: **18,5–20,3 MB WorkingSet** (**5,3–7,15 MB Private**), `0.0%` CPU idle.
  - `deck-studio.exe` fechado: **0 MB de WebView2 residual**.
  - Rede (`iperf3` 300 Mbps UDP ×3): **0% de perda** e **0 descartes** na placa `Realtek Ethernet 3`.
- **Botões e Ações Validados:** navegação (`OBS` / `>` / `<`), controles OBS WebSocket v5, 4 fontes de sensores (`AIDA64`, `LibreHardwareMonitor`, `HWiNFO64`, `Windows Nativo + NVML`), clima Open-Meteo e Smart Profiles WinEvent (`0%` polling).
- **Testes:** `cargo test --workspace` **16/16 passaram** (incluindo servidor mock OBS WebSocket v5) + simulação ao vivo em todas as teclas.

## Rollback para o App Oficial
Caso precise restaurar a inicialização original, execute [rollback/restore_stream_dock_ajazz.reg](../../rollback/restore_stream_dock_ajazz.reg).
