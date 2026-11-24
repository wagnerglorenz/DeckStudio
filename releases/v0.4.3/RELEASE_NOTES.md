# DeckStudio v0.4.3 — Defesa em Profundidade Completa

- `SetDefaultDllDirectories(LOAD_LIBRARY_SEARCH_DEFAULT_DIRS)` no boot do motor e checagem autenticada (`GET /api/health` + Bearer token) no `is_engine_running()` do Tauri.
- `POST /api/system/update/install` pinado ao `last_update_check` do servidor.
- Validação de `obs.host` + timeout de 5s no cliente OBS WebSocket, limpeza de `press_times` na troca de perfil e `XmlReader` endurecido em `sensor-doctor.ps1`.
