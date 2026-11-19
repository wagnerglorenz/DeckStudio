Set sh = CreateObject("WScript.Shell")
sh.CurrentDirectory = "C:\Users\wagne\workspace\StreamDeck"
sh.Run """C:\Users\wagne\workspace\StreamDeck\.venv\Scripts\pythonw.exe"" -m ajazz_monitor.main --config config.yaml --watchdog", 0, False
