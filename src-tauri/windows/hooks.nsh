; Installer hooks (see bundle.windows.nsis.installerHooks in tauri.conf.json).
;
; The installer puts rune-studio-cli.exe next to the app. These hooks add that folder to the
; user's PATH on install and take it out again on uninstall, so `rune-studio-cli` works from any
; new terminal. They only touch the per-user PATH (no admin rights), keep every other entry
; as it is, and are safe to run again (an update runs them too).

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
