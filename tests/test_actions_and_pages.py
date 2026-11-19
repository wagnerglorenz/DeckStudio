"""Testes unitários para PageManager e ActionDispatcher."""
from ajazz_monitor.actions import ActionDispatcher
from ajazz_monitor.pages import PageManager


class _DummyDevice:
    def __init__(self) -> None:
        self.cleared = 0
        self.brightness_history: list[int] = []

    def clear_all(self) -> None:
        self.cleared += 1

    def set_brightness(self, val: int) -> None:
        self.brightness_history.append(val)


class _DummyObs:
    def __init__(self) -> None:
        self.calls: list[tuple] = []
        self.recording = False

    def start_record(self) -> bool:
        self.recording = True
        self.calls.append(("start_record",))
        return True

    def stop_record(self) -> bool:
        self.recording = False
        self.calls.append(("stop_record",))
        return True

    def toggle_record_pause(self) -> bool:
        self.calls.append(("toggle_record_pause",))
        return True

    def toggle_record(self) -> bool:
        self.calls.append(("toggle_record",))
        return True

    def toggle_stream(self) -> bool:
        self.calls.append(("toggle_stream",))
        return True

    def set_scene(self, name: str) -> bool:
        self.calls.append(("scene", name))
        return True

    def toggle_mute(self, inp: str) -> bool:
        self.calls.append(("toggle_mute", inp))
        return True

    def save_replay(self) -> bool:
        self.calls.append(("save_replay",))
        return True


def test_page_navigation_and_obs_actions():
    pages = PageManager({"monitor": {}, "obs": {}})
    dev = _DummyDevice()
    obs = _DummyObs()
    changed = []
    disp = ActionDispatcher(pages, dev, obs, on_page_changed=lambda: changed.append(pages.current_name))

    assert pages.current_name == "monitor"
    disp.dispatch({"page": "next"})
    assert pages.current_name == "obs"
    assert dev.cleared == 1
    assert changed == ["obs"]

    disp.dispatch({"page": "prev"})
    assert pages.current_name == "monitor"

    disp.dispatch({"brightness": "+20"})
    assert dev.brightness_history[-1] == 80

    disp.dispatch({"obs": "toggle_record"})
    disp.dispatch({"obs": "toggle_mute", "input": "Mic/Aux"})
    disp.dispatch({"obs": "scene", "name": "Câmera"})
    assert obs.calls == [
        ("toggle_record",),
        ("toggle_mute", "Mic/Aux"),
        ("scene", "Câmera"),
    ]


def test_record_two_tap_confirmation_and_pause():
    pages = PageManager({"monitor": {}, "obs": {}})
    dev = _DummyDevice()
    obs = _DummyObs()
    disp = ActionDispatcher(pages, dev, obs)

    # Pausar quando parado não faz nada
    disp.dispatch({"obs": "toggle_record_pause"})
    assert obs.calls == []

    # 1º toque arma confirmação mas não inicia gravação
    disp.dispatch({"obs": "record_confirm"})
    assert disp.record_confirm_armed is True
    assert obs.calls == []

    # 2º toque confirma e inicia gravação
    disp.dispatch({"obs": "record_confirm"})
    assert disp.record_confirm_armed is False
    assert obs.recording is True
    assert obs.calls == [("start_record",)]

    # Pausar durante gravação funciona
    disp.dispatch({"obs": "toggle_record_pause"})
    assert obs.calls[-1] == ("toggle_record_pause",)

    # Parar também exige 2 toques
    disp.dispatch({"obs": "record_confirm"})
    assert obs.recording is True
    disp.dispatch({"obs": "record_confirm"})
    assert obs.recording is False
    assert obs.calls[-1] == ("stop_record",)
