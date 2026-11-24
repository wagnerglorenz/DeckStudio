# DeckStudio Design System (`v0.2.0`)

Para facilitar evoluções rápidas sem regressões visuais nem CSS duplicado, a interface desktop (`deckstudio/apps/studio`) é organizada sobre **tokens semânticos** e **componentes tipados reutilizáveis**.

## 1. Estrutura de Arquivos
- [tokens.css](../deckstudio/apps/studio/src/design-system/tokens.css) — tokens CSS (`:root`) para fundos, superfícies elevadas, bordas, tipografia, marca, estados semânticos, glows, espaçamentos e raios.
- [components.tsx](../deckstudio/apps/studio/src/design-system/components.tsx) — primitivos React tipados (`DeckLogo`, `Button`, `StatusPill`, `Card`, `KpiCard`, `Field`).
- [AppIcons.tsx](../deckstudio/apps/studio/src/design-system/AppIcons.tsx) — catálogo vetorial de logos de apps populares (`AppLogoIcon`, `ICON_CATALOG`: OBS Studio, Discord, Spotify, Chrome, Edge, VS Code, Steam, Twitch, YouTube, NVIDIA, Terminal, Pastas e ícones de mídia/OBS).
- [index.ts](../deckstudio/apps/studio/src/design-system/index.ts) — ponto único de exportação e importação automática de `tokens.css`.
- [styles.css](../deckstudio/apps/studio/src/styles.css) — folhas de layout e composição consumindo exclusivamente variáveis `--ds-*`.

## 2. Catálogo de Tokens (`--ds-*`)

| Categoria | Tokens Principais | Uso |
| :--- | :--- | :--- |
| **Superfícies** | `--ds-bg-canvas` (`#0B0F19`), `--ds-bg-shell` (`#0D1320`), `--ds-bg-topbar` (`#0E1422`), `--ds-bg-card` (`#0F1727`), `--ds-bg-elevated` (`#141F33`), `--ds-bg-input` (`#162032`) | Hierarquia visual escura de alto contraste sem fadiga |
| **Marca** | `--ds-brand-cyan` (`#22D3EE`), `--ds-brand-sky` (`#38BDF8`), `--ds-brand-blue` (`#2563EB`), `--ds-brand-emerald` (`#22C55E`) | Seleção ativa, logomarca, botões primários e indicadores positivos |
| **Estados** | `--ds-semantic-success*`, `--ds-semantic-danger*`, `--ds-semantic-warning` | Feedback de gravação OBS, status do engine, descartes NIC e alertas |
| **Tipografia** | `--ds-font-sans`, `--ds-font-mono` | UI principal em Segoe UI/system-ui e IDs/códigos em Consolas |
| **Espaçamento & Raios** | `--ds-space-1` (`4px`) .. `--ds-space-6` (`24px`), `--ds-radius-xs` .. `--ds-radius-pill` | Alinhamento uniforme de cards, pills, botões e grade 6×3 |

## 3. Componentes Primitivos (`components.tsx`)
- **`<DeckLogo size={number} />`**: marca vetorial oficial (squircle escuro, grade 2×3 ciano + faixa lateral verde/ciano formando "D").
- **`<Button variant="default | primary | success | danger" />`**: ação padronizada.
- **`<StatusPill dot="green | cyan | red | gray" accent? />`**: indicador de estado na barra superior, tela Sobre e alertas.
- **`<Card title? action? />`**: contêiner de seção reutilizado nas abas Temas, Perfis, Fontes, Saúde e Sobre.
- **`<KpiCard label value hint? accent? />`**: cartões de métrica na aba Saúde.
- **`<Field label />`**: rótulo maiúsculo padronizado para formulários.

## 4. Regras de Modificação
1. **Proibido hardcodar hexadecimais novos** direto em layouts de página sem justificar ou registrar em `tokens.css`.
2. **Novas telas ou painéis** devem compor `Card`, `StatusPill`, `Button`, `KpiCard` e `Field` antes de criar classes novas.
3. Alterações estruturais no Design System exigem atualização deste documento e do diagrama [tree](../docs/archify/tree/deckstudio-tree.html).
