# ajazz-monitor (StreamDeck)

Substituto leve em Python para o **Stream Dock AJAZZ** no **AJAZZ AKP153E** (`VID 0x0300`, `PID 0x1010`), sem navegador embutido e sem perda de pacotes na placa de rede.

## Estrutura
- `ajazz_monitor/device_proto.py` — protocolo HID vendor-defined (chunks de 512 bytes) e `sanitize_image()`.
- `ajazz_monitor/device.py` — fila limitada (`max_tiles_per_s`), cache de hash e mapa físico verificado (`6×3`).
- `ajazz_monitor/sensors.py` — leitura da memória compartilhada `AIDA64_SensorValues`, contadores de rede Windows (`GetIfTable`), cache de 30 s e vigia opcional do AIDA64.
- `ajazz_monitor/render.py` — renderização Pillow com sparklines e botões de estado.
- `ajazz_monitor/obs.py` — cliente OBS WebSocket v5 (`obsws-python`) com reconexão automática.
- `ajazz_monitor/main.py` — loop principal e ícone na bandeja (`pystray`).
- `config.yaml` — layout das páginas `monitor` e `obs`.

## Executar
Certifique-se de que o **Stream Dock AJAZZ** oficial esteja fechado:

```powershell
.\.venv\Scripts\python.exe -m ajazz_monitor.main --config config.yaml
```

Para usar senha no OBS WebSocket, defina a variável de ambiente `OBS_WS_PASSWORD`.
Para ativar a vigia do AIDA64 (`schtasks` na tarefa `"AIDA64 AutoStart"`), adicione `--watchdog`.

## Testes
```powershell
.\.venv\Scripts\python.exe -m pytest -v
```

## Voltar para o app oficial (Rollback)
1. Clique com o botão direito no ícone da bandeja do `ajazz-monitor` e escolha **Sair**.
2. Caso tenha desativado o app oficial da inicialização, reimporte o `.reg` salvo e devolva `iniciar-ponte.vbs` para a pasta Inicializar.
3. Abra `C:\Program Files (x86)\Stream Dock AJAZZ Global\Stream Dock AJAZZ.exe`.
