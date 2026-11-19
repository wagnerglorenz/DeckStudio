# Regras Obrigatórias — Releases Versionadas, Archify (Análise de Impacto) e Licenciamento

Toda alteração neste repositório deve seguir estas regras sem exceção:

## 1. Análise de Impacto via Archify (Antes e Depois de Alterar Código)
- **Antes de editar código:** consulte os diagramas aplicáveis em `docs/archify/` através da matriz em `docs/archify/README.md` para avaliar impacto em:
  - Protocolo HID 512 bytes (`hid_proto.rs`, `device.rs`)
  - Despacho de threads nativas (`akp-reader`) para o runtime Tokio (`actions.rs`, `state.rs`)
  - Prioridade e fallback das 4 fontes de sensores + standby do `nvml.dll` (`sensors/*`)
  - Contratos IPC (`127.0.0.1:19876`) e schema `AppConfig` (`deck-core`)
  - Consumo de RAM, zero desvios/descartes na placa Realtek e latência UDP
- **Após alterações estruturais ou de fluxo:** atualize o(s) `docs/archify/<tipo>/candidate.json` impactado(s) e revalide obrigatoriamente com:
  ```powershell
  node "C:\Users\wagne\.gemini\config\plugins\archify\skills\archify\bin\archify.mjs" finalize <tipo> "docs/archify/<tipo>/candidate.json" "docs/archify/<tipo>/deckstudio-<tipo>.html" --repo-root "c:\Users\wagne\workspace\StreamDeck" --quality showcase --json
  ```

## 2. Releases Obrigatórias em Cada Versão
- Toda nova versão (`vX.Y.Z`) deve:
  1. Atualizar a versão em `deckstudio/crates/deck-core/Cargo.toml`, `deckstudio/crates/deck-engine/Cargo.toml`, `deckstudio/apps/studio/package.json` e `deckstudio/apps/studio/src-tauri/tauri.conf.json`.
  2. Registrar mudanças no `CHANGELOG.md` e criar `releases/vX.Y.Z/RELEASE_NOTES.md`.
  3. Atualizar a documentação de compatibilidade (`docs/COMPATIBILITY.md` e tela **Sobre** em `App.tsx`) sempre que suporte a hardware/modelos ou protocolos mudar.
  4. Executar `.\scripts\release.ps1 -Version "<X.Y.Z>" -PublishGitHub` para rodar `cargo test --workspace`, gerar binários release, empacotar `DeckStudio-vX.Y.Z-win-x64.zip` + `SHA256SUMS.txt`, criar a tag anotada `vX.Y.Z` e publicar o Release no GitHub.

## 3. Posicionamento, Licença e Privacidade no GitHub
- O **DeckStudio** é uma alternativa **100% gratuita e open-source sob Licença MIT** ([LICENSE](../../LICENSE)).
- Não colocar nomes de marcas de hardware específicas (como "AJAZZ AKP153E") no título principal da janela/app — manter apenas **DeckStudio** no título e listar os modelos compatíveis na aba **Sobre / About** e em [docs/COMPATIBILITY.md](../../docs/COMPATIBILITY.md).
- Enquanto não houver instrução explícita em contrário, manter o repositório remoto no GitHub como **Private**.
