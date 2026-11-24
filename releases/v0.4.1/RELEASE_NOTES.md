# DeckStudio v0.4.1 — Hardening Completo de Segurança

## Correções Aplicadas
- Autenticação Bearer por sessão (`ipc.token`) + CORS restrito ao Tauri + validação de `Host`.
- Auto-Updater fail-closed (HTTPS oficial fixo, SHA-256 obrigatório, staging exclusivo sem TOCTOU, `DETACHED_PROCESS`, `nsis-hooks.nsh` sem `/T`).
- Confinamento de caminhos em `/api/backup/restore` e `/api/import/ajazz` (bloqueio de UNC/SMB), redação de `obs.password` em IPC e `Documentos`.
- `ShellExecuteW` no lugar de `cmd /C start`, anti-SSRF em Webhooks e LibreHWM, carregamento qualificado de DLLs/binários em `System32`, cópia segura de Shared Memory + `is_finite()`, CSP ativa no WebView2 e validação de `custom_image_b64`.
