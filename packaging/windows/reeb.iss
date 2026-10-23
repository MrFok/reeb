#ifndef AppVersion
  #error AppVersion must be supplied by scripts/build-windows.ps1
#endif

[Setup]
AppId=com.mrfok.reeb
AppName=reeb
AppVersion={#AppVersion}
AppPublisher=Ricky Fok
AppPublisherURL=https://github.com/MrFok/reeb
AppSupportURL=https://github.com/MrFok/reeb/issues
AppUpdatesURL=https://github.com/MrFok/reeb/releases
DefaultDirName={localappdata}\Programs\reeb
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
DisableProgramGroupPage=yes
DisableDirPage=auto
UninstallDisplayIcon={app}\reeb.exe
SetupIconFile=..\..\assets\ui\reeb.ico
OutputDir=..\..\dist
OutputBaseFilename=reeb-setup-windows-x64
VersionInfoVersion={#AppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no
CloseApplicationsFilter=reeb.exe

[Tasks]
Name: "desktopicon"; Description: "Create a desktop shortcut"; Flags: unchecked

[Files]
Source: "..\..\dist\reeb.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\dist\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\dist\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\dist\THIRD-PARTY-NOTICES.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\dist\KBSIM-LICENSE.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\dist\CC0-LICENSE.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\dist\SOUND-NOTICES.txt"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\dist\sound-packs.json"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\..\dist\PLUS-JAKARTA-SANS-LICENSE.txt"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\reeb"; Filename: "{app}\reeb.exe"
Name: "{autodesktop}\reeb"; Filename: "{app}\reeb.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\reeb.exe"; Description: "Open reeb"; Flags: nowait postinstall skipifsilent
