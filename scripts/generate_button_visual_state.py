from local_coder import ask_local_model

prompt = """
Você é um desenvolvedor Rust sênior trabalhando no projeto DeckStudio (um motor de StreamDeck em Rust).
Escreva a função completa `button_visual_state` em Rust, com tratamento abrangente e dinâmico para operações do OBS Studio.

Assinaturas e Estruturas disponíveis:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ObsSceneItem {
    pub id: i64,
    pub name: String,
    pub enabled: bool,
}

pub struct ObsState {
    pub connected: bool,
    pub recording: bool,
    pub paused: bool,
    pub streaming: bool,
    pub virtual_cam: bool,
    pub studio_mode: bool,
    pub current_scene: String,
    pub scenes: Vec<String>,
    pub audio_inputs: Vec<String>,
    pub muted: HashMap<String, bool>,
    pub scene_items: HashMap<String, Vec<ObsSceneItem>>,
    pub all_sources: Vec<String>,
}

pub enum ActionConfig {
    Obs {
        op: String,
        name: Option<String>,
        input: Option<String>,
        scene: Option<String>,
        source: Option<String>,
    },
    // outros variantes...
}
```

Requisitos da função:
```rust
pub fn button_visual_state(
    tile: &TileConfig,
    obs: &ObsState,
    cfg_pages: &BTreeMap<String, BTreeMap<String, TileConfig>>,
) -> (String, String, String)
```
O retorno é `(visual_state: String, subtitle: String, title: String)`.
Regras que a função deve cobrir com precisão:
1. Se `tile.action` for `ActionConfig::Obs` e `!obs.connected`, retornar imediatamente `("disabled".into(), "OFF".into(), title)`.
2. Avaliar tanto `tile.state` explícito quanto a operação `op` em `tile.action = ActionConfig::Obs { op, name, input, scene, source, .. }`:
   - Gravação (`op == "toggle_record"` ou `state == Some("record")`):
     - Se `obs.recording && obs.paused`: ("warning", "PAUSADO", "GRAVANDO")
     - Se `obs.recording`: ("active", "PARAR", "GRAVANDO")
     - Caso contrário: ("idle", "CLIQUE", title se não vazio senão "Gravar")
   - Pausa de gravação (`op == "toggle_record_pause"` ou `state == Some("record_pause")`):
     - Se `!obs.recording`: ("disabled", "PARADO", title)
     - Se `obs.paused`: ("warning", "RETOMAR", "PAUSADO")
     - Caso contrário: ("idle", "REC ATIVO", title)
   - Transmissão/Live (`op == "toggle_stream"` ou `state == Some("stream")`):
     - Se `obs.streaming`: ("active", "LIVE", title)
     - Senão: ("idle", "OFF", title)
   - Câmera Virtual (`op == "toggle_virtualcam"`):
     - Se `obs.virtual_cam`: ("active", "CAM ON", title)
     - Senão: ("idle", "OFF", title)
   - Studio Mode (`op == "toggle_studio_mode"`):
     - Se `obs.studio_mode`: ("active", "STUDIO", title)
     - Senão: ("idle", "OFF", title)
   - Mute de áudio (`op == "toggle_mute"` ou `state == Some("mute")`):
     - Descobrir nome da entrada em `input` ou padrão "Mic/Aux".
     - Se `obs.muted.get(inp) == Some(&true)`: ("warning", "MUDO", title)
     - Senão: ("active", "SOM ATIVO", title)
   - Troca de Cena (`op == "scene"`):
     - Se `name` coincidir com `obs.current_scene` (comparação case-insensitive ou exata): ("active", "ATIVA", title)
     - Senão: ("idle", sub, title)
   - Ativação/Desativação de Fontes OBS (`op == "toggle_source"` ou `"show_source"` ou `"hide_source"`):
     - Procurar a fonte `source` na cena especificada `scene` ou em `obs.current_scene`.
     - Procurar em `obs.scene_items.get(target_scene)` pelo item onde `it.name.eq_ignore_ascii_case(src)`.
     - Se encontrado e `it.enabled`: ("active", if sub.is_empty() { "VISÍVEL".into() } else { sub }, title)
     - Se encontrado e `!it.enabled`: ("idle", if sub.is_empty() { "OCULTO".into() } else { sub }, title)
     - Se não encontrado na cena atual: ("idle", sub, title)
3. Preservar suporte para pastas (`tile.kind == "folder"` define sub = "PASTA" se vazio).
4. Se `title` for vazio e houver páginas em `cfg_pages`, usar primeira chave.
5. Retornar no fallback `("idle".into(), sub, title)`.

Gere APENAS o código Rust da função `pub fn button_visual_state`, limpo, idiomático, eficiente e pronto para compilar.
"""

print(ask_local_model(prompt))
