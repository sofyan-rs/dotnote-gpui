; Inno Setup script for the Windows installer, built by .github/workflows/release.yml:
;   iscc /DAppVersion=1.0.0 /DSourceExe=target\release\dotnote.exe packaging\windows\dotnote.iss
;
; Installs per user (no admin) into %LOCALAPPDATA%\Programs\Dotnote, so the
; in-app updater can replace Dotnote.exe there without asking for admin rights.

#ifndef AppVersion
  #error Pass /DAppVersion=x.y.z
#endif
#ifndef SourceExe
  #define SourceExe "..\..\target\release\dotnote.exe"
#endif

[Setup]
; Never change AppId: Windows uses it to find an existing install to upgrade.
AppId={{81B9876A-6CBF-4A1D-89FE-AD0DCF679931}
AppName=Dotnote
AppVersion={#AppVersion}
AppPublisher=Sofyan
AppPublisherURL=https://github.com/sofyan-rs/dotnote-gpui
AppSupportURL=https://github.com/sofyan-rs/dotnote-gpui/issues
AppUpdatesURL=https://github.com/sofyan-rs/dotnote-gpui/releases
DefaultDirName={autopf}\Dotnote
DefaultGroupName=Dotnote
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
SetupIconFile=dotnote.ico
UninstallDisplayIcon={app}\Dotnote.exe
UninstallDisplayName=Dotnote
OutputDir=..\..
OutputBaseFilename=Dotnote-{#AppVersion}-windows-x64-setup
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceExe}"; DestDir: "{app}"; DestName: "Dotnote.exe"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\Dotnote"; Filename: "{app}\Dotnote.exe"
Name: "{autodesktop}\Dotnote"; Filename: "{app}\Dotnote.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\Dotnote.exe"; Description: "{cm:LaunchProgram,Dotnote}"; Flags: nowait postinstall skipifsilent

[UninstallDelete]
; Left behind by the in-app updater until the next launch.
Type: files; Name: "{app}\Dotnote.exe.old"
