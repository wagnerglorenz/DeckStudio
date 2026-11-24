# DeckStudio v0.4.2 — Hardening Pós-Auditoria (Run-2)

- Corrigidos todos os pontos práticos levantados na segunda auditoria (`/security-audit run-2`): pânico UTF-8 e chunked OOM no LibreHWM, bypasses de SSRF em Webhooks, validação estrita de `Host` e `BCryptGenRandom`, bloqueio de `..`/`%2e` no updater, alinhamento de buffer em `sample_net_octets`, `"$SYSDIR\taskkill.exe"` no NSIS, `form-action 'none'` no CSP e limite de 512 KiB no upload de imagem.
