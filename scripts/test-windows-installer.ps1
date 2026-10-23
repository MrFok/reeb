param([string]$Installer = 'dist/reeb-setup-windows-x64.exe')

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw 'Installer verification requires Windows.'
}
$installerPath = (Resolve-Path -LiteralPath $Installer).Path
$registry = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\com.mrfok.reeb_is1'
$shortcut = Join-Path ([Environment]::GetFolderPath('Programs')) 'reeb.lnk'
if ((Test-Path -LiteralPath $registry) -or (Test-Path -LiteralPath $shortcut)) {
    throw 'Use a clean test account; reeb is already installed or has a Start Menu shortcut.'
}
$work = Join-Path ([IO.Path]::GetTempPath()) ('reeb-installer-test-' + [guid]::NewGuid())
$installDir = Join-Path $work 'app'
New-Item -ItemType Directory -Path $work | Out-Null
$uninstaller = Join-Path $installDir 'unins000.exe'
try {
    # Install twice to exercise stable AppId and upgrade-in-place behavior.
    foreach ($attempt in 1..2) {
        $arguments = @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/SP-',
            "/DIR=`"$installDir`"", "/LOG=`"$work/install-$attempt.log`"")
        $process = Start-Process -FilePath $installerPath -ArgumentList $arguments -Wait -PassThru
        if ($process.ExitCode -ne 0) { throw "Installer failed with code $($process.ExitCode). Logs: $work" }
        $expected = @('reeb.exe', 'README.md', 'LICENSE', 'THIRD-PARTY-NOTICES.txt',
            'KBSIM-LICENSE.txt', 'CC0-LICENSE.txt', 'SOUND-NOTICES.txt',
            'sound-packs.json', 'PLUS-JAKARTA-SANS-LICENSE.txt')
        foreach ($file in $expected) {
            $installed = Join-Path $installDir $file
            $source = Join-Path (Split-Path $installerPath -Parent) $file
            if (-not (Test-Path -LiteralPath $installed -PathType Leaf)) { throw "Missing installed file: $file" }
            if ((Get-FileHash -LiteralPath $installed).Hash -ne (Get-FileHash -LiteralPath $source).Hash) {
                throw "Installed content differs: $file"
            }
        }
        if (-not (Test-Path -LiteralPath $registry)) { throw 'Missing per-user uninstall entry.' }
        if (-not (Test-Path -LiteralPath $shortcut)) { throw 'Missing Start Menu shortcut.' }
        $entry = Get-ItemProperty -LiteralPath $registry
        if ($entry.InstallLocation.TrimEnd('\') -ne $installDir) { throw 'Upgrade changed the install location.' }
        if (-not (Test-Path -LiteralPath $uninstaller)) { throw 'Missing uninstaller.' }
    }
    $demo = Join-Path $work 'demo.wav'
    $process = Start-Process -FilePath (Join-Path $installDir 'reeb.exe') -ArgumentList @('--render-demo', "`"$demo`"") -Wait -PassThru
    if ($process.ExitCode -ne 0 -or -not (Test-Path -LiteralPath $demo)) { throw 'Installed executable demo failed.' }
    if ((Get-Item -LiteralPath $demo).Length -le 44) { throw 'Installed executable produced empty audio.' }
} finally {
    if (Test-Path -LiteralPath $uninstaller) {
        $process = Start-Process -FilePath $uninstaller -ArgumentList @('/VERYSILENT', '/SUPPRESSMSGBOXES', '/NORESTART', '/SP-') -Wait -PassThru
        if ($process.ExitCode -ne 0) { throw "Uninstall failed. Inspect $installDir" }
    }
}
if ((Test-Path -LiteralPath $registry) -or (Test-Path -LiteralPath $shortcut) -or
    (Test-Path -LiteralPath (Join-Path $installDir 'reeb.exe'))) {
    throw "Uninstall left the application, registry entry or shortcut. Inspect $work"
}
Write-Output "Verified per-user install, repeated install, installed demo and uninstall. Logs: $work"
