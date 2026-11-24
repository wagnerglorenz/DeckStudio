!macro NSIS_HOOK_PREINSTALL
  ; Fecha processos ativos antes de atualizar os binários (sem /T e com caminho qualificado em System32)
  nsExec::ExecToLog '"$SYSDIR\taskkill.exe" /F /IM deck-engine.exe'
  nsExec::ExecToLog '"$SYSDIR\taskkill.exe" /F /IM DeckStudio.exe'
  Sleep 400
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; Registra o motor leve (deck-engine.exe) para iniciar com o Windows e acender o Stream Deck no boot
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "DeckStudioEngine" '"$INSTDIR\deck-engine.exe"'
  ; Inicia (ou reinicia após atualização) o motor leve na bandeja
  Exec '"$INSTDIR\deck-engine.exe"'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; Encerra processos em execução antes de desinstalar e limpa entradas de inicialização
  nsExec::ExecToLog '"$SYSDIR\taskkill.exe" /F /IM deck-engine.exe'
  nsExec::ExecToLog '"$SYSDIR\taskkill.exe" /F /IM DeckStudio.exe'
  Sleep 400
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "DeckStudioEngine"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "DeckStudioDash"
!macroend
