; Post-install: register Slime Smol USB-serial INFs (paths match bundle.resources: $INSTDIR\resources\windows-driver\)
!macro NSIS_HOOK_POSTINSTALL
  IfFileExists "$INSTDIR\resources\windows-driver\slime_smol_tracker.inf" 0 skip_slime_infs
  IfFileExists "$INSTDIR\resources\windows-driver\slime_smol_receiver.inf" 0 skip_slime_infs

  ExecWait '"$SYSDIR\pnputil.exe" /add-driver "$INSTDIR\resources\windows-driver\slime_smol_tracker.inf" /install' $0
  IntCmp $0 0 +3
    MessageBox MB_ICONSTOP|MB_OK "FoxDock：无法注册 Slime Smol Tracker USB 串口驱动（pnputil 退出码 $0）。$\n正式环境请使用已签名的目录包（.cat）。"
    Abort

  ExecWait '"$SYSDIR\pnputil.exe" /add-driver "$INSTDIR\resources\windows-driver\slime_smol_receiver.inf" /install' $0
  IntCmp $0 0 +3
    MessageBox MB_ICONSTOP|MB_OK "FoxDock：无法注册 Slime Smol Receiver USB 串口驱动（pnputil 退出码 $0）。$\n正式环境请使用已签名的目录包（.cat）。"
    Abort

  skip_slime_infs:
!macroend
