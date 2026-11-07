; Instalador de Parlo para Windows (Inno Setup 6). ADR-0006, docs/packaging.md.
; Lo invoca packaging/windows/build.ps1:
;   ISCC.exe /DAppVersion=0.0.1 /DSourceDir=<staging> /DOutputDir=<dist> packaging\windows\parlo.iss
; Instalación por usuario (sin UAC) por defecto; el diálogo permite elegir «para todos los usuarios».
; Los datos del usuario (%APPDATA%\Parlo) y el llavero NO se borran al desinstalar.

#ifndef AppVersion
  #define AppVersion "0.0.0"
#endif
#ifndef SourceDir
  #define SourceDir "..\..\dist\staging\windows"
#endif
#ifndef OutputDir
  #define OutputDir "..\..\dist"
#endif

[Setup]
; AppId fijo: identifica la app para actualizar/desinstalar. No cambiarlo nunca entre versiones.
AppId={{6B0F3C1E-8E4A-4D57-9A61-2F0D5C7B9E21}
AppName=Parlo
AppVersion={#AppVersion}
AppVerName=Parlo {#AppVersion}
AppPublisher=Parlo
DefaultDirName={autopf}\Parlo
DefaultGroupName=Parlo
DisableProgramGroupPage=yes
PrivilegesRequired=lowest
PrivilegesRequiredOverridesAllowed=dialog
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0
OutputDir={#OutputDir}
OutputBaseFilename=Parlo-{#AppVersion}-windows-x86_64-setup
SetupIconFile=parlo.ico
UninstallDisplayIcon={app}\parlo.exe
UninstallDisplayName=Parlo
Compression=lzma2/max
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes

[Languages]
Name: "es"; MessagesFile: "compiler:Languages\Spanish.isl"
Name: "en"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "{cm:CreateDesktopIcon}"; GroupDescription: "{cm:AdditionalIcons}"; Flags: unchecked

[Files]
Source: "{#SourceDir}\parlo.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\README.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "{#SourceDir}\licenses\*"; DestDir: "{app}\licenses"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{autoprograms}\Parlo"; Filename: "{app}\parlo.exe"
Name: "{autodesktop}\Parlo"; Filename: "{app}\parlo.exe"; Tasks: desktopicon

[Run]
Filename: "{app}\parlo.exe"; Description: "{cm:LaunchProgram,Parlo}"; Flags: nowait postinstall skipifsilent
