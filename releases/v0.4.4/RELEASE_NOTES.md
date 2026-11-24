# DeckStudio v0.4.4 — Correções Finais Run-3

- Prova mútua `GET /api/ping?c=<nonce>` entre `deck-studio` e `deck-engine` (evita vazamento do Bearer token em *port-squatting*).
- Filtro anti-SSRF completo em Webhooks com `reqwest::Url` + `lookup_host` + `ClientBuilder::resolve`.
- Alinhamento UTF-8 em `parse_aida_xml`, limpeza de `obs.password` em `put_config` ao trocar host/porta, bloqueio de UNC em `spawn_program` e `-MaximumRedirection 0` em `sensor-doctor.ps1`.
