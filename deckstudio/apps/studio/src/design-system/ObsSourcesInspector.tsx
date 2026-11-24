import React, { useState, useMemo } from "react";

export interface ObsSourcesInspectorProps {
  sources: string[];
  selectedSource?: string;
  onSelectSource: (sourceName: string) => void;
  activeScene?: string;
  onRefresh?: () => void;
  isRefreshing?: boolean;
}

function getSourceBadge(source: string): { label: string; emoji: string; color: string } {
  const s = source.toLowerCase();
  if (
    s.includes("cam") ||
    s.includes("webcam") ||
    s.includes("brio") ||
    s.includes("camera") ||
    s.includes("câmera") ||
    s.includes("facecam")
  ) {
    return { label: "CAM", emoji: "📷", color: "#22D3EE" };
  }
  if (
    s.includes("som") ||
    s.includes("audio") ||
    s.includes("áudio") ||
    s.includes("mic") ||
    s.includes("headset") ||
    s.includes("jogo") ||
    s.includes("game")
  ) {
    return { label: "ÁUDIO", emoji: "🔊", color: "#38BDF8" };
  }
  if (
    s.includes("tela") ||
    s.includes("screen") ||
    s.includes("display") ||
    s.includes("monitor") ||
    s.includes("janela") ||
    s.includes("window") ||
    s.includes("acc") ||
    s.includes("foco")
  ) {
    return { label: "TELA", emoji: "🖥️", color: "#A78BFA" };
  }
  if (
    s.includes("logo") ||
    s.includes("img") ||
    s.includes("image") ||
    s.includes("foto") ||
    s.includes("overlay") ||
    s.includes("borda") ||
    s.includes("banner")
  ) {
    return { label: "IMAGEM", emoji: "🖼️", color: "#F472B6" };
  }
  return { label: "FONTE", emoji: "📦", color: "#94A3B8" };
}

export const ObsSourcesInspector: React.FC<ObsSourcesInspectorProps> = ({
  sources,
  selectedSource,
  onSelectSource,
  activeScene,
  onRefresh,
  isRefreshing,
}) => {
  const [filter, setFilter] = useState("");

  const filteredSources = useMemo(() => {
    if (!filter.trim()) return sources;
    const term = filter.toLowerCase();
    return sources.filter((s) => s.toLowerCase().includes(term));
  }, [sources, filter]);

  return (
    <div
      style={{
        marginTop: 10,
        padding: 10,
        borderRadius: 8,
        background: "var(--ds-bg-surface)",
        border: "1px solid var(--ds-border)",
      }}
    >
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: 8 }}>
        <div style={{ fontSize: 11, fontWeight: 600, color: "var(--ds-text)" }}>
          Inspetor de Fontes OBS {activeScene ? `(${activeScene})` : ""}
        </div>
        <div style={{ display: "flex", gap: 6, alignItems: "center" }}>
          <span style={{ fontSize: 10, color: "var(--ds-text-secondary)" }}>
            {filteredSources.length} de {sources.length}
          </span>
          {onRefresh && (
            <button
              type="button"
              className="btn"
              style={{ fontSize: 10, padding: "2px 6px" }}
              onClick={onRefresh}
              disabled={isRefreshing}
              title="Atualizar lista de fontes do OBS"
            >
              {isRefreshing ? "⏳" : "🔄"}
            </button>
          )}
        </div>
      </div>

      <input
        className="input"
        style={{ fontSize: 11, padding: "4px 8px", marginBottom: 8, width: "100%" }}
        placeholder="🔍 Filtrar fontes pelo nome..."
        value={filter}
        onChange={(e) => setFilter(e.target.value)}
      />

      {filteredSources.length === 0 ? (
        <div
          style={{
            padding: "16px 8px",
            textAlign: "center",
            fontSize: 11,
            color: "var(--ds-text-secondary)",
            background: "var(--ds-bg-subtle)",
            borderRadius: 6,
          }}
        >
          {sources.length === 0
            ? "Nenhuma fonte detectada no OBS. Verifique se o OBS está aberto e conectado."
            : "Nenhuma fonte corresponde ao filtro digitado."}
        </div>
      ) : (
        <div
          style={{
            display: "grid",
            gridTemplateColumns: "repeat(auto-fill, minmax(130px, 1fr))",
            gap: 6,
            maxHeight: 180,
            overflowY: "auto",
            paddingRight: 2,
          }}
        >
          {filteredSources.map((source) => {
            const isSelected = selectedSource?.toLowerCase() === source.toLowerCase();
            const badge = getSourceBadge(source);

            return (
              <button
                key={source}
                type="button"
                onClick={() => onSelectSource(source)}
                style={{
                  display: "flex",
                  alignItems: "center",
                  justifyContent: "space-between",
                  gap: 6,
                  padding: "6px 8px",
                  borderRadius: 6,
                  cursor: "pointer",
                  textAlign: "left",
                  fontSize: 11,
                  background: isSelected ? "rgba(56, 189, 248, 0.15)" : "var(--ds-bg-subtle)",
                  border: isSelected ? "1px solid var(--ds-accent)" : "1px solid var(--ds-border)",
                  color: isSelected ? "var(--ds-text)" : "var(--ds-text-secondary)",
                  transition: "all 0.15s ease",
                }}
                title={`Clique para vincular a fonte "${source}" a esta tecla`}
              >
                <div style={{ display: "flex", alignItems: "center", gap: 5, minWidth: 0, overflow: "hidden" }}>
                  <span style={{ fontSize: 13, flexShrink: 0 }}>{badge.emoji}</span>
                  <span
                    style={{
                      whiteSpace: "nowrap",
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      fontWeight: isSelected ? 600 : 400,
                    }}
                  >
                    {source}
                  </span>
                </div>
                <span
                  style={{
                    fontSize: 8,
                    fontWeight: 700,
                    padding: "1px 4px",
                    borderRadius: 3,
                    background: `${badge.color}25`,
                    color: badge.color,
                    border: `1px solid ${badge.color}50`,
                    flexShrink: 0,
                  }}
                >
                  {badge.label}
                </span>
              </button>
            );
          })}
        </div>
      )}
    </div>
  );
};
