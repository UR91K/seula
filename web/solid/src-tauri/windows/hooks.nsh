; NSIS hooks for the Seula installer (ADR-0068, ADR-0069).
;
; Tauri's installer already asks the user to close the shell. It knows nothing about the
; daemon or the plugin scanner worker, which are separate programs (ADR-0048, ADR-0004).
; The shell starts the daemon and leaves it running after its window closes, so on an
; upgrade or an uninstall both are normally running, and a running executable cannot be
; overwritten or deleted.
;
; Killed by image name, not by path: the install path can hold characters that do not
; survive being quoted into a command line. The cost is that any seula.exe on the machine
; goes, which on an end user's machine is only this one.

!macro SEULA_STOP_BACKGROUND_PROCESSES
  ; The worker first, so a scan in progress is not left supervising nothing.
  nsExec::Exec '"$SYSDIR\taskkill.exe" /F /T /IM vst-meta.exe'
  Pop $0
  nsExec::Exec '"$SYSDIR\taskkill.exe" /F /T /IM seula.exe'
  Pop $0
  ; Let Windows release the file handles before the files are replaced or removed.
  Sleep 1000
!macroend

!macro NSIS_HOOK_PREINSTALL
  !insertmacro SEULA_STOP_BACKGROUND_PROCESSES
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  !insertmacro SEULA_STOP_BACKGROUND_PROCESSES
!macroend
