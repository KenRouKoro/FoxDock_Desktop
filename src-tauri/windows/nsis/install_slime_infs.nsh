; Post-install: 在安装程序已提升权限的前提下注册 INF（与 bundle.resources 路径一致）
;
; Tauri 在 installMode=perMachine 时使用 RequestExecutionLevel admin，本宏在 Install Section 末尾执行，
; 与安装程序同一进程、同一管理员令牌，无需 Start-Process -Verb RunAs（否则会二次 UAC）。
; 此处选用 64 位 pnputil：在 32 位 NSIS 上优先 $WINDIR\Sysnative\...，避免 WoW64 指向 SysWOW64。
;
; 若仍失败，多为未签名目录包策略限制，而非权限不足（见 docs/DRIVER_SIGNING.md）。
!macro NSIS_HOOK_POSTINSTALL
  IfFileExists "$INSTDIR\resources\windows-driver\slime_smol_tracker.inf" 0 skip_slime_infs
  IfFileExists "$INSTDIR\resources\windows-driver\slime_smol_receiver.inf" 0 skip_slime_infs

  Push $R0
  Push $R1

  IfFileExists "$WINDIR\Sysnative\pnputil.exe" 0 use_system32_pnputil
    StrCpy $R0 "$WINDIR\Sysnative\pnputil.exe"
    Goto pnputil_path_done
  use_system32_pnputil:
    StrCpy $R0 "$WINDIR\System32\pnputil.exe"
  pnputil_path_done:

  ExecWait '"$R0" /add-driver "$INSTDIR\resources\windows-driver\slime_smol_tracker.inf" /install' $R1
  IntCmp $R1 0 +2
    MessageBox MB_ICONEXCLAMATION|MB_OK "FoxDock：未能自动注册 Slime Smol Tracker 的 INF（退出码 $R1）。$\n无签名目录包时属常见情况；应用内仍会显示设备名称。$\n若需在设备管理器中显示友好名，请见文档「无 CA 时的 INF」。"

  ExecWait '"$R0" /add-driver "$INSTDIR\resources\windows-driver\slime_smol_receiver.inf" /install' $R1
  IntCmp $R1 0 +2
    MessageBox MB_ICONEXCLAMATION|MB_OK "FoxDock：未能自动注册 Slime Smol Receiver 的 INF（退出码 $R1）。$\n无签名目录包时属常见情况；应用内仍会显示设备名称。$\n若需在设备管理器中显示友好名，请见文档「无 CA 时的 INF」。"

  Pop $R1
  Pop $R0

  skip_slime_infs:
!macroend
