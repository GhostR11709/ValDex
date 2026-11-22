[Setup]
AppId={{7C4E9A2B-1D58-4F63-A0E7-6B91C3D84F20}
AppName=ValDex
; Keep this equal to the version in rust/launch/Cargo.toml.
AppVersion=0.1.1
AppPublisher=ValDex
DefaultDirName={localappdata}\ValDex
DisableDirPage=yes
DisableProgramGroupPage=yes
DisableReadyPage=yes
PrivilegesRequired=lowest
OutputDir=..\dist
OutputBaseFilename=ValDex-Setup
SetupIconFile=..\vivi.ico
UninstallDisplayIcon={app}\vivi-player.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
CloseApplications=yes
RestartApplications=no

[Files]
Source: "..\dist\staging\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\NOTICE"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autodesktop}\ValDex"; Filename: "{app}\vivi-start.exe"; WorkingDir: "{app}"; IconFilename: "{app}\vivi.ico"; Comment: "Open ValDex"
Name: "{autoprograms}\ValDex"; Filename: "{app}\vivi-start.exe"; WorkingDir: "{app}"; IconFilename: "{app}\vivi.ico"; Comment: "Open ValDex"

[Run]
Filename: "{app}\vivi-start.exe"; Description: "Open ValDex"; Flags: nowait postinstall skipifsilent
