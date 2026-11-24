from local_coder import ask_local_model

prompt = """
Você é um desenvolvedor React + TypeScript experiente desenvolvendo a interface do DeckStudio.
Crie um componente React moderno chamado `ObsSourcesInspector.tsx` para listar e inspecionar visualmente as fontes do OBS Studio, permitindo que o usuário veja as fontes disponíveis como cartões interativos e vincule uma fonte à tecla selecionada com 1 clique.

Especificações:
1. Props do componente:
```typescript
export interface ObsSourcesInspectorProps {
  sources: string[];
  selectedSource?: string;
  onSelectSource: (sourceName: string) => void;
  activeScene?: string;
  onRefresh?: () => void;
  isRefreshing?: boolean;
}
```
2. Heurística visual para ícones e badges com base no nome da fonte:
   - Contém "cam", "webcam", "brio", "câmera", "camera", "facecam" -> badge "CAM", ícone/emoji 📷, cor "#22D3EE"
   - Contém "som", "audio", "mic", "headset", "jogo", "game", "desktop" -> badge "ÁUDIO", emoji 🔊, cor "#38BDF8"
   - Contém "tela", "screen", "display", "monitor", "janela", "window", "acc", "foco" -> badge "TELA", emoji 🖥️, cor "#A78BFA"
   - Contém "logo", "img", "image", "foto", "overlay", "borda", "banner" -> badge "IMAGEM", emoji 🖼️, cor "#F472B6"
   - Padrão -> badge "FONTE", emoji 📦, cor "#94A3B8"
3. Layout e Design System:
   - Usar as variáveis CSS do Design System do DeckStudio: `var(--ds-bg-surface)`, `var(--ds-bg-subtle)`, `var(--ds-border)`, `var(--ds-border-active)`, `var(--ds-text)`, `var(--ds-text-secondary)`, `var(--ds-accent)`.
   - Grid compacto de fontes: 2 a 3 colunas (`gridTemplateColumns: "repeat(auto-fill, minmax(130px, 1fr))"`), gap de 6px.
   - Cada cartão deve ser um botão clicável com cantos arredondados (radius 8px), padding suave, exibindo o emoji/ícone, o nome da fonte (com truncamento ellipsis se muito longo) e o badge colorido.
   - Estado selecionado (`selectedSource === sourceName`): borda destacada `var(--ds-accent)` (ciano/azul neon) e leve fundo translúcido (`rgba(56, 189, 248, 0.12)`).
   - Incluir uma barra de busca rápida (`input`) no topo para filtrar fontes pelo nome quando houver muitas fontes.
   - Se a lista estiver vazia, exibir mensagem acolhedora informando que o OBS precisa estar aberto e conectado.
4. Código completo em TypeScript com React funcional (`import React, { useState, useMemo } from 'react';`).

Gere APENAS o código do arquivo `ObsSourcesInspector.tsx` completo e pronto para ser salvo e importado.
"""

print(ask_local_model(prompt))
