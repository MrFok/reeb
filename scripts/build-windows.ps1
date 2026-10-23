param([string]$Tag = '')

$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest

if ([Environment]::OSVersion.Platform -ne [PlatformID]::Win32NT) {
    throw 'Build on Windows with Rust x64 MSVC and the Visual Studio C++ Build Tools/Windows SDK.'
}

Push-Location (Split-Path $PSScriptRoot -Parent)
try {
    $rustVersion = rustc -vV
    if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect the Rust toolchain.' }
    if ($rustVersion -notcontains 'host: x86_64-pc-windows-msvc') {
        throw 'Use the x86_64-pc-windows-msvc Rust toolchain.'
    }
    if ($env:CARGO_BUILD_TARGET) { throw 'Unset CARGO_BUILD_TARGET to build for the host toolchain.' }

    $metadataJson = cargo metadata --locked --no-deps --format-version 1
    if ($LASTEXITCODE -ne 0) { throw 'Cannot read locked package metadata.' }
    $metadata = $metadataJson | ConvertFrom-Json
    if ($metadata.packages.Count -ne 1 -or $metadata.packages[0].name -ne 'reeb') {
        throw 'Expected the standalone reeb package.'
    }
    $version = $metadata.packages[0].version
    if ($version -notmatch '^\d+\.\d+\.\d+$') { throw 'Expected a stable three-part package version.' }
    if ($Tag -and $Tag -cne "v$version") { throw "Tag must match Cargo.toml: v$version" }

    $noticeFiles = [ordered]@{
        'LICENSE' = 'LICENSE'
        'THIRD-PARTY-NOTICES.txt' = 'THIRD-PARTY-NOTICES.txt'
        'assets/KBSIM-LICENSE.txt' = 'KBSIM-LICENSE.txt'
        'assets/CC0-LICENSE.txt' = 'CC0-LICENSE.txt'
        'assets/SOUND-NOTICES.txt' = 'SOUND-NOTICES.txt'
        'assets/sound-packs.json' = 'sound-packs.json'
        'assets/ui/OFL.txt' = 'PLUS-JAKARTA-SANS-LICENSE.txt'
    }
    $packageReadme = Join-Path $PSScriptRoot 'package-README.md'
    foreach ($path in @($packageReadme) + @($noticeFiles.Keys)) {
        if (-not (Test-Path -LiteralPath $path -PathType Leaf) -or (Get-Item -LiteralPath $path).Length -eq 0) {
            throw "Required package file is missing or empty: $path"
        }
    }
    $license = Get-Content -LiteralPath LICENSE -Raw
    if (-not $license.Contains('Permission is hereby granted, free of charge, to any person obtaining a copy') -or
        -not $license.Contains('THE SOFTWARE IS PROVIDED "AS IS"')) {
        throw 'The root LICENSE must contain the MIT license text.'
    }

    $dependencies = cargo tree --locked --target x86_64-pc-windows-msvc --edges normal,build --prefix none --format '{p}'
    if ($LASTEXITCODE -ne 0) { throw 'Cannot inspect locked Windows dependencies.' }
    $notices = Get-Content -LiteralPath THIRD-PARTY-NOTICES.txt -Raw
    $missing = @($dependencies | ForEach-Object {
        if ($_ -match '^(\S+) v(\S+)') {
            $name = $Matches[1]
            $dependencyVersion = $Matches[2]
            if ($name -ne 'reeb' -and -not $notices.Contains("$name $dependencyVersion")) {
                "$name $dependencyVersion"
            }
        }
    } | Sort-Object -Unique)
    if ($missing.Count -gt 0) { throw "Regenerate third-party notices; missing Windows/build entries: $($missing -join ', ')" }

    $buildJson = cargo build --release --locked --bin reeb --message-format=json-render-diagnostics
    if ($LASTEXITCODE -ne 0) { throw 'Cargo release build failed.' }
    $artifacts = @($buildJson | ForEach-Object { $_ | ConvertFrom-Json } | Where-Object {
        $_.reason -eq 'compiler-artifact' -and $_.target.name -eq 'reeb' -and $_.target.kind -contains 'bin'
    })
    $executable = (Resolve-Path -LiteralPath (Join-Path $metadata.target_directory 'release/reeb.exe')).Path
    if ($artifacts.Count -ne 1 -or $artifacts[0].executable -ne $executable) {
        throw 'Cargo must produce release/reeb.exe in its configured target directory; check Cargo target configuration.'
    }
    $reader = [IO.BinaryReader]::new([IO.File]::OpenRead($executable))
    try {
        if ($reader.ReadUInt16() -ne 0x5A4D) { throw 'Invalid Windows executable.' }
        $reader.BaseStream.Position = 0x3C
        $header = $reader.ReadInt32()
        $reader.BaseStream.Position = $header
        if ($reader.ReadUInt32() -ne 0x4550 -or $reader.ReadUInt16() -ne 0x8664) {
            throw 'Expected a Windows x64 executable.'
        }
    } finally {
        $reader.Dispose()
    }

    New-Item -ItemType Directory -Path dist -Force | Out-Null
    Copy-Item -LiteralPath $executable -Destination dist/reeb.exe -Force
    Copy-Item -LiteralPath $packageReadme -Destination dist/README.md -Force
    foreach ($entry in $noticeFiles.GetEnumerator()) {
        Copy-Item -LiteralPath $entry.Key -Destination (Join-Path dist $entry.Value) -Force
    }
    $noticePaths = @($noticeFiles.Values | ForEach-Object { Join-Path dist $_ })
    $packagePaths = @('dist/reeb.exe', 'dist/README.md') + $noticePaths
    Compress-Archive -LiteralPath $packagePaths -DestinationPath dist/reeb-windows-x64.zip -CompressionLevel Optimal -Force
    Compress-Archive -LiteralPath $noticePaths -DestinationPath dist/reeb-notices.zip -CompressionLevel Optimal -Force

    $releaseFiles = @('reeb.exe', 'reeb-windows-x64.zip', 'reeb-notices.zip', 'LICENSE')
    $checksums = @($releaseFiles | ForEach-Object {
        $hash = (Get-FileHash -LiteralPath (Join-Path dist $_) -Algorithm SHA256).Hash.ToLowerInvariant()
        "$hash  $_"
    })
    [IO.File]::WriteAllLines((Join-Path (Get-Location).Path 'dist/SHA256SUMS'), $checksums, [Text.UTF8Encoding]::new($false))
    Write-Output "Built Reeb $version from $executable; release files are in dist/."
    Write-Output 'Recommended: reeb-windows-x64.zip. Keep LICENSE and reeb-notices.zip with a standalone reeb.exe download.'
} finally {
    Pop-Location
}
