!macro NSIS_HOOK_POSTINSTALL
  ; Notify Explorer of the updated icon resources and the shortcuts that use them.
  System::Call 'shell32::SHChangeNotify(i 0x2000, i 0x0005, w "$INSTDIR\${MAINBINARYNAME}.exe", p 0)'
  System::Call 'shell32::SHChangeNotify(i 0x2000, i 0x0005, w "$SMPROGRAMS\${PRODUCTNAME}.lnk", p 0)'
  System::Call 'shell32::SHChangeNotify(i 0x2000, i 0x0005, w "$DESKTOP\${PRODUCTNAME}.lnk", p 0)'
!macroend
