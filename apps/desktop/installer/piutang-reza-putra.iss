#ifndef AppVersion
  #define AppVersion "0.1.0-beta.1"
#endif
#ifndef AppExe
  #error AppExe must point to the built desktop executable
#endif

[Setup]
AppId={{3C25B80E-BD01-4A89-ACB3-350940E986CF}
AppName=Piutang Reza Putra
AppVersion={#AppVersion}
AppPublisher=Reza Putra
DefaultDirName={autopf}\Piutang Reza Putra
DefaultGroupName=Piutang Reza Putra
UninstallDisplayName=Piutang Reza Putra
UninstallDisplayIcon={app}\Piutang Reza Putra.exe
SetupIconFile=..\assets\piutang-reza-putra.ico
OutputDir=..\..\..\dist\desktop
OutputBaseFilename=Piutang-Reza-Putra-Setup-{#AppVersion}
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
PrivilegesRequired=admin
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible

[Tasks]
Name: "desktopicon"; Description: "Buat shortcut di desktop"; Flags: unchecked

[Files]
Source: "{#AppExe}"; DestDir: "{app}"; DestName: "Piutang Reza Putra.exe"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Piutang Reza Putra"; Filename: "{app}\Piutang Reza Putra.exe"
Name: "{autodesktop}\Piutang Reza Putra"; Filename: "{app}\Piutang Reza Putra.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\Piutang Reza Putra.exe"; Description: "Jalankan Piutang Reza Putra"; Flags: nowait postinstall skipifsilent
