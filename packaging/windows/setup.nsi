; The FastEmu setup for Windows. packaging/windows/package.ps1 builds it
; with NSIS 3.13, the defines first:
;
;   makensis /V2 /WX /DVERSION=0.2.0 /DSTAGE=<folder> /DOUTFILE=<setup.exe> setup.nsi
;
; STAGE is the folder of the portable zip: fastemu.exe, fastemu.com, the
; README and the licenses. The setup installs them for the current user,
; with no administrator rights, into %LOCALAPPDATA%\Programs\FastEmu. It
; adds a Start menu shortcut, a desktop shortcut when its box on the last
; page is ticked, FastEmu under Open with for .jar and .jad files, and an
; entry in Settings > Apps. /S installs silently, with no desktop
; shortcut. Uninstall.exe removes all of it, and /S uninstalls silently.
; The games and settings in %APPDATA%\fastemu stay.

!ifndef VERSION
  !error "Give the version: /DVERSION=X.Y.Z"
!endif
!ifndef STAGE
  !error "Give the folder of the files to install: /DSTAGE=<folder>"
!endif
!ifndef OUTFILE
  !error "Give the setup's path: /DOUTFILE=<setup.exe>"
!endif

Unicode true
ManifestDPIAware true
SetCompressor /SOLID lzma
RequestExecutionLevel user

!define ID "io.github.fzn0x.fastemu"
!define UNINSTALL_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${ID}"
!define CLASSES "Software\Classes"
!define COMMAND '"$INSTDIR\fastemu.exe" "%1"'

Name "FastEmu"
OutFile "${OUTFILE}"
InstallDir "$LOCALAPPDATA\Programs\FastEmu"
; An update installs where the last version went.
InstallDirRegKey HKCU "${UNINSTALL_KEY}" "InstallLocation"
BrandingText "FastEmu ${VERSION}"

!include MUI2.nsh
!include FileFunc.nsh

!define MUI_ICON "${__FILEDIR__}\..\icons\fastemu.ico"
!define MUI_UNICON "${__FILEDIR__}\..\icons\fastemu.ico"
!define MUI_ABORTWARNING
!define MUI_FINISHPAGE_RUN "$INSTDIR\fastemu.exe"
!define MUI_FINISHPAGE_RUN_TEXT "Open FastEmu"
; The last page's second box makes the desktop shortcut.
!define MUI_FINISHPAGE_SHOWREADME
!define MUI_FINISHPAGE_SHOWREADME_TEXT "Add a shortcut to the desktop"
!define MUI_FINISHPAGE_SHOWREADME_NOTCHECKED
!define MUI_FINISHPAGE_SHOWREADME_FUNCTION DesktopShortcut

!insertmacro MUI_PAGE_DIRECTORY
!insertmacro MUI_PAGE_INSTFILES
!insertmacro MUI_PAGE_FINISH
!insertmacro MUI_UNPAGE_CONFIRM
!insertmacro MUI_UNPAGE_INSTFILES
!insertmacro MUI_LANGUAGE "English"

; The version Explorer's Properties show for the setup.
VIProductVersion "${VERSION}.0"
VIAddVersionKey /LANG=${LANG_ENGLISH} "ProductName" "FastEmu"
VIAddVersionKey /LANG=${LANG_ENGLISH} "ProductVersion" "${VERSION}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "FileVersion" "${VERSION}"
VIAddVersionKey /LANG=${LANG_ENGLISH} "FileDescription" "FastEmu setup"
VIAddVersionKey /LANG=${LANG_ENGLISH} "CompanyName" "The FastEmu contributors"
VIAddVersionKey /LANG=${LANG_ENGLISH} "LegalCopyright" "The FastEmu contributors, GPL-3.0-or-later"

; Windows locks the file of a program that runs, so FastEmu cannot be
; replaced or removed while it is open. CloseFastEmu asks to close it, or
; with /S closes it without asking: close-fastemu.ps1 asks its windows to
; close and ends whatever is left after 10 seconds.
!macro CLOSE_FASTEMU UN
Function ${UN}CloseFastEmu
  StrCpy $R0 0
  check:
    ; A running program's file cannot be opened for writing.
    IfFileExists "$INSTDIR\fastemu.exe" 0 check_com
    ClearErrors
    FileOpen $0 "$INSTDIR\fastemu.exe" a
    IfErrors running
    FileClose $0
  check_com:
    IfFileExists "$INSTDIR\fastemu.com" 0 free
    ClearErrors
    FileOpen $0 "$INSTDIR\fastemu.com" a
    IfErrors running
    FileClose $0
  free:
    Return
  running:
    IntCmp $R0 0 ask close close
  ask:
    MessageBox MB_OKCANCEL|MB_ICONEXCLAMATION "FastEmu is open. Choose OK to close it and go on, or Cancel to stop." /SD IDOK IDOK close
    Abort
  close:
    IntOp $R0 $R0 + 1
    IntCmp $R0 4 gave_up 0 gave_up
    InitPluginsDir
    File "/oname=$PLUGINSDIR\close-fastemu.ps1" "${__FILEDIR__}\close-fastemu.ps1"
    nsExec::Exec 'powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "$PLUGINSDIR\close-fastemu.ps1" "$INSTDIR"'
    Pop $0
    Sleep 500
    Goto check
  gave_up:
    MessageBox MB_OK|MB_ICONSTOP "FastEmu is still open. Close it, then try again." /SD IDOK
    Abort
FunctionEnd
!macroend
!insertmacro CLOSE_FASTEMU ""
!insertmacro CLOSE_FASTEMU "un."

; FastEmu joins the Open with list of .jar or .jad files. It does not
; become what opens them: Java's own association stays as it is.
!macro OPEN_WITH EXT DESCRIPTION
  WriteRegStr HKCU "${CLASSES}\${ID}.${EXT}" "" "${DESCRIPTION}"
  WriteRegStr HKCU "${CLASSES}\${ID}.${EXT}\DefaultIcon" "" "$INSTDIR\fastemu.exe,0"
  WriteRegStr HKCU "${CLASSES}\${ID}.${EXT}\shell\open\command" "" '${COMMAND}'
  WriteRegStr HKCU "${CLASSES}\.${EXT}\OpenWithProgids" "${ID}.${EXT}" ""
  WriteRegStr HKCU "${CLASSES}\Applications\fastemu.exe\SupportedTypes" ".${EXT}" ""
!macroend

; Tells Explorer that the file types changed.
!macro ASSOCIATIONS_CHANGED
  System::Call 'shell32::SHChangeNotify(i 0x08000000, i 0, p 0, p 0)'
!macroend

Function DesktopShortcut
  CreateShortcut "$DESKTOP\FastEmu.lnk" "$INSTDIR\fastemu.exe"
FunctionEnd

Section "-FastEmu"
  Call CloseFastEmu
  SetOutPath "$INSTDIR"
  File "${STAGE}\fastemu.exe"
  File "${STAGE}\fastemu.com"
  File "${STAGE}\README.md"
  File "${STAGE}\LICENSE"
  File "${STAGE}\THIRD_PARTY_NOTICES.md"
  WriteUninstaller "$INSTDIR\Uninstall.exe"

  CreateShortcut "$SMPROGRAMS\FastEmu.lnk" "$INSTDIR\fastemu.exe"

  WriteRegStr HKCU "${CLASSES}\Applications\fastemu.exe" "FriendlyAppName" "FastEmu"
  WriteRegStr HKCU "${CLASSES}\Applications\fastemu.exe\shell\open\command" "" '${COMMAND}'
  !insertmacro OPEN_WITH "jar" "Java ME game"
  !insertmacro OPEN_WITH "jad" "Java ME game descriptor"
  !insertmacro ASSOCIATIONS_CHANGED

  ; Settings > Apps.
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayName" "FastEmu"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayVersion" "${VERSION}"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "DisplayIcon" "$INSTDIR\fastemu.exe,0"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "Publisher" "The FastEmu contributors"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "URLInfoAbout" "https://fastemu.functionx.dev"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "InstallLocation" "$INSTDIR"
  WriteRegStr HKCU "${UNINSTALL_KEY}" "UninstallString" '"$INSTDIR\Uninstall.exe"'
  WriteRegStr HKCU "${UNINSTALL_KEY}" "QuietUninstallString" '"$INSTDIR\Uninstall.exe" /S'
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoModify" 1
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "NoRepair" 1
  ${GetSize} "$INSTDIR" "/S=0K" $0 $1 $2
  IntFmt $0 "0x%08X" $0
  WriteRegDWORD HKCU "${UNINSTALL_KEY}" "EstimatedSize" "$0"
SectionEnd

; Removes what the setup added, and only that: the .jar and .jad keys
; keep the values of other programs.
Section "Uninstall"
  Call un.CloseFastEmu
  Delete "$SMPROGRAMS\FastEmu.lnk"
  Delete "$DESKTOP\FastEmu.lnk"

  DeleteRegKey HKCU "${CLASSES}\${ID}.jar"
  DeleteRegKey HKCU "${CLASSES}\${ID}.jad"
  DeleteRegValue HKCU "${CLASSES}\.jar\OpenWithProgids" "${ID}.jar"
  DeleteRegValue HKCU "${CLASSES}\.jad\OpenWithProgids" "${ID}.jad"
  DeleteRegKey HKCU "${CLASSES}\Applications\fastemu.exe"
  !insertmacro ASSOCIATIONS_CHANGED
  DeleteRegKey HKCU "${UNINSTALL_KEY}"

  Delete "$INSTDIR\fastemu.exe"
  Delete "$INSTDIR\fastemu.com"
  Delete "$INSTDIR\README.md"
  Delete "$INSTDIR\LICENSE"
  Delete "$INSTDIR\THIRD_PARTY_NOTICES.md"
  Delete "$INSTDIR\Uninstall.exe"
  ; Only when empty: whatever else is in the folder stays.
  RMDir "$INSTDIR"
SectionEnd
