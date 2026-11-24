# DeckStudio v0.4.5

## Novidades e Melhorias
- **Qualidade & Confiabilidade:** zero warnings em `cargo clippy --workspace --all-targets -- -D warnings`, `deck-studio` (`src-tauri`) e `tsc --noEmit`.
- **Limpeza de Código:** remoção de código morto, conversões redundantes e aplicação de padrões idiomáticos (`checked_div`, `div_ceil`, `is_multiple_of`, `sort_by_key`).
- Inclui todas as correções de segurança das 3 rodadas completas de `/security-audit`, além de temas, Mini Criador, Copiar/Colar, Segurar Tecla, Mixer WASAPI, Cronômetro/Pomodoro e Backup/Auto-Restauração.
