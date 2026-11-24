"""Testes unitários para sensors.py, render.py e config.py."""
from pathlib import Path

from legacy_monitor.config import load_config
from legacy_monitor.device_proto import sanitize_image
from legacy_monitor.render import (
    BUILTIN_THEMES,
    render_button_tile,
    render_clock_tile,
    render_compact_health_tile,
    render_cpu_chip_tile,
    render_nav_circle_tile,
    render_net_strip_tile,
    render_obs_status_tile,
    render_ram_gauge_tile,
    render_sensor_tile,
    render_weather_tile,
    temp_color,
    usage_color,
)
from legacy_monitor.sensors import SensorStore, parse_aida_xml


def test_parse_aida_xml_and_format():
    xml = (
        "<temp><id>TCPU</id><label>CPU</label><value>57.6</value></temp>"
        "<pwr><id>PGPU1</id><label>GPU</label><value>150.3</value></pwr>"
    )
    rows = parse_aida_xml(xml)
    assert len(rows) == 2

    store = SensorStore()
    values = {"TCPU": 57.6, "NET_DL": 120.4, "NET_UL": 45.2}
    txt, num = store.format_sensor("TCPU", values, unit="°C", decimals=0)
    assert txt == "58°C"
    assert num == 58.0

    net_txt, net_num = store.format_sensor("NET_BOTH", values)
    assert net_txt == "120/45.2"
    assert abs(net_num - 165.6) < 0.01


def test_render_tiles_sanitize_cleanly():
    for theme in BUILTIN_THEMES.values():
        tiles = [
            render_sensor_tile("CPU °C", "58", unit="°", history=[50, 52, 55, 58], vmin=0, vmax=100, theme=theme, raw_value=58.0, group="cpu", is_temp=True),
            render_sensor_tile("GPU", "40", unit="%", history=[20, 40], vmin=0, vmax=100, theme=theme, raw_value=40.0, group="gpu", card_style="ring"),
            render_button_tile("Gravar", subtitle="REC", state="active", theme=theme),
            render_button_tile("PARAR?", subtitle="CONFIRMA", state="warning", theme=theme),
            render_clock_tile("20:15", "18/11", theme=theme),
            render_obs_status_tile("REC", "2.Jogo", theme=theme),
            render_weather_tile(25.3, 20.2, 30.8, "Sta Cruz Sul", 3, theme=theme),
            render_compact_health_tile("CPU", 17.0, 66.0, "#22D3EE", theme=theme),
            render_nav_circle_tile("prev", theme=theme),
            render_nav_circle_tile("next", theme=theme),
            render_net_strip_tile("12.4Mb/s", "0Kb/s", theme=theme),
            render_cpu_chip_tile("52°C", temp_value=52.0, theme=theme),
            render_ram_gauge_tile(34.0, theme=theme),
        ]
        for tile in tiles:
            assert len(sanitize_image(tile)) <= 10240


def test_temp_color_scale():
    assert temp_color(48.0) == "#22C55E"
    assert temp_color(68.0) == "#EAB308"
    assert temp_color(80.0) == "#EF4444"
    assert usage_color(40.0) == "#22C55E"
    assert usage_color(72.0) == "#EAB308"
    assert usage_color(90.0) == "#EF4444"


def test_load_default_config():
    cfg = load_config(Path(__file__).resolve().parent.parent / "config.yaml")
    assert cfg.device.brightness == 60
    assert "monitor" in cfg.pages
    assert "obs" in cfg.pages
    assert cfg.pages["monitor"]["1,1"]["id"] == "TCPU"
