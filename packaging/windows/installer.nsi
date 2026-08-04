; NSIS Installer Script for Yntra Platform (Windows)
; Requires NSIS 3.0+

!define PRODUCT_NAME "Yntra Platform"
!define PRODUCT_VERSION "0.1.0"
!define PRODUCT_PUBLISHER "Yntra Technologies"
!define PRODUCT_WEB_SITE "https://yntra.se"
!define PRODUCT_DIR_REGKEY "Software\Microsoft\Windows\CurrentVersion\App Paths\yntra-ui.exe"
!define PRODUCT_UNINST_KEY "Software\Microsoft\Windows\CurrentVersion\Uninstall\${PRODUCT_NAME}"
!define PRODUCT_UNINST_ROOT_KEY "HKLM"

SetCompressor lzma

; MUI 1.67 compatible ------
!include "MUI2.nsh"

; MUI Settings
!define MUI_ABORTWARNING
!define MUI_ICON "${NSISDIR}\Contrib\Graphics\Icons\modern-install.ico"
!define MUI_UNICON "${NSISDIR}\Contrib\Graphics\Icons\modern-uninstall.ico"

; Welcome page
!insertmacro MUI_PAGE_WELCOME
; Directory page
!insertmacro MUI_PAGE_DIRECTORY
; Instfiles page
!insertmacro MUI_PAGE_INSTFILES
; Finish page
!define MUI_FINISHPAGE_RUN "$INSTDIR\yntra-ui.exe"
!insertmacro MUI_PAGE_FINISH

; Uninstaller pages
!insertmacro MUI_PAGE_UNINSTCONFIRM
!insertmacro MUI_PAGE_INSTFILES

; Language files
!insertmacro MUI_LANGUAGE "English"

Name "${PRODUCT_NAME} ${PRODUCT_VERSION}"
OutFile "..\..\target\release\YntraPlatform-Setup.exe"
InstallDir "$PROGRAMFILES64\YntraPlatform"
InstallDirRegKey HKLM "${PRODUCT_DIR_REGKEY}" ""
ShowInstDetails show
ShowUnInstDetails show

Section "MainSection" SEC01
  SetOutPath "$INSTDIR"
  SetOverwrite ifnewer
  File "..\..\target\release\yntra-ui.exe"
  
  ; Create Desktop Shortcut
  CreateShortCut "$DESKTOP\Yntra Platform.lnk" "$INSTDIR\yntra-ui.exe" "" "$INSTDIR\yntra-ui.exe" 0
  
  ; Create Start Menu Shortcuts
  CreateDirectory "$SMPROGRAMS\Yntra Platform"
  CreateShortCut "$SMPROGRAMS\Yntra Platform\Yntra Platform.lnk" "$INSTDIR\yntra-ui.exe" "" "$INSTDIR\yntra-ui.exe" 0
  CreateShortCut "$SMPROGRAMS\Yntra Platform\Uninstall Yntra Platform.lnk" "$INSTDIR\uninst.exe"
SectionEnd

Section -Post
  WriteUninstaller "$INSTDIR\uninst.exe"
  WriteRegStr HKLM "${PRODUCT_DIR_REGKEY}" "" "$INSTDIR\yntra-ui.exe"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "DisplayName" "$(etc_Name) ${PRODUCT_NAME}"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "UninstallString" "$INSTDIR\uninst.exe"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "DisplayVersion" "${PRODUCT_VERSION}"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "URLInfoAbout" "${PRODUCT_WEB_SITE}"
  WriteRegStr ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}" "Publisher" "${PRODUCT_PUBLISHER}"
SectionEnd

Function un.onUninstSuccess
  HideWindow
  MessageBox MB_OK|MB_ICONINFORMATION "Yntra Platform was successfully uninstalled."
FunctionEnd

Function un.onInit
  MessageBox MB_YESNO|MB_ICONQUESTION "Are you sure you want to uninstall Yntra Platform?" IDYES +2
  Abort
FunctionEnd

Section Uninstall
  Delete "$INSTDIR\yntra-ui.exe"
  Delete "$INSTDIR\uninst.exe"
  Delete "$DESKTOP\Yntra Platform.lnk"
  Delete "$SMPROGRAMS\Yntra Platform\Yntra Platform.lnk"
  Delete "$SMPROGRAMS\Yntra Platform\Uninstall Yntra Platform.lnk"
  RMDir "$SMPROGRAMS\Yntra Platform"
  RMDir "$INSTDIR"

  DeleteRegKey ${PRODUCT_UNINST_ROOT_KEY} "${PRODUCT_UNINST_KEY}"
  DeleteRegKey HKLM "${PRODUCT_DIR_REGKEY}"
  SetAutoClose true
SectionEnd
