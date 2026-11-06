
!macro NSIS_HOOK_POSTINSTALL
  ${If} ${FileExists} "$INSTDIR\rune-studio-cli.exe"
    nsExec::ExecToLog `powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command "$$d='$INSTDIR'; $$p=[Environment]::GetEnvironmentVariable('Path','User'); if ($$null -eq $$p) { $$p='' }; if (($$p -split ';') -notcontains $$d) { [Environment]::SetEnvironmentVariable('Path', ($$p.TrimEnd(';') + ';' + $$d).TrimStart(';'), 'User') }"`
    Pop $0
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  nsExec::ExecToLog `powershell -NoProfile -NonInteractive -ExecutionPolicy Bypass -Command "$$d='$INSTDIR'; $$p=[Environment]::GetEnvironmentVariable('Path','User'); if ($$null -ne $$p) { $$kept = ($$p -split ';') | Where-Object { $$_ -and $$_ -ne $$d }; [Environment]::SetEnvironmentVariable('Path', ($$kept -join ';'), 'User') }"`
  Pop $0
!macroend
