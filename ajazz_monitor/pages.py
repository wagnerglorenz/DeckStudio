"""Gerenciamento de páginas a partir do config.yaml."""
from __future__ import annotations

from typing import Any


class PageManager:
    def __init__(self, pages: dict[str, dict[str, dict[str, Any]]]) -> None:
        self._pages = pages
        self._names = list(pages.keys())
        if not self._names:
            raise ValueError("Nenhuma página configurada")
        self._index = 0

    @property
    def current_name(self) -> str:
        return self._names[self._index]

    @property
    def current_items(self) -> dict[str, dict[str, Any]]:
        return self._pages[self.current_name]

    def set_page(self, target: str) -> str:
        if target == "next":
            self._index = (self._index + 1) % len(self._names)
        elif target == "prev":
            self._index = (self._index - 1) % len(self._names)
        elif target in self._pages:
            self._index = self._names.index(target)
        else:
            raise KeyError(f"Página desconhecida: {target}")
        return self.current_name
