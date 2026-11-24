param([string]$Destination = '')
$ErrorActionPreference = 'Stop'
$repoRoot = Split-Path -Parent $PSScriptRoot
if (-not $Destination) { $Destination = Join-Path $repoRoot 'dist\PocketOS' }
Push-Location $repoRoot
try {
    cargo build --release -p pocketos-launcher --offline
    if ($LASTEXITCODE -ne 0) { throw 'Rust build failed' }
    New-Item -ItemType Directory -Force -Path $Destination | Out-Null
    Copy-Item -LiteralPath (Join-Path $repoRoot 'target\release\PocketOS.exe') -Destination (Join-Path $Destination 'pocketos-core.exe') -Force
    $framework = Join-Path $env:WINDIR 'Microsoft.NET\Framework64\v4.0.30319'
    & (Join-Path $framework 'csc.exe') /nologo /codepage:65001 /target:winexe /optimize+ /platform:x64 ("/out:" + (Join-Path $Destination 'PocketOS.exe')) ("/r:" + (Join-Path $framework 'WPF\PresentationFramework.dll')) ("/r:" + (Join-Path $framework 'WPF\PresentationCore.dll')) ("/r:" + (Join-Path $framework 'WPF\WindowsBase.dll')) /r:System.Xaml.dll (Join-Path $repoRoot 'launcher-ui\PocketLoader.cs')
    if ($LASTEXITCODE -ne 0) { throw 'Loader UI build failed' }
    foreach ($binary in @('PocketOS.exe','pocketos-core.exe')) {
        $file = Join-Path (Resolve-Path $Destination).Path $binary
        $bytes = [IO.File]::ReadAllBytes($file)
        $offset = [BitConverter]::ToInt32($bytes,0x3c)
        if ($offset -lt 0 -or $offset+24 -gt $bytes.Length -or [BitConverter]::ToUInt32($bytes,$offset) -ne 0x4550) { throw 'Invalid PE executable' }
        [BitConverter]::GetBytes([uint16]([BitConverter]::ToUInt16($bytes,$offset+22) -bor 0x0400)).CopyTo($bytes,$offset+22)
        [IO.File]::WriteAllBytes($file,$bytes)
    }
    Copy-Item -LiteralPath (Join-Path $repoRoot 'README.md') -Destination $Destination -Force
    Copy-Item -LiteralPath (Join-Path $repoRoot 'docs') -Destination $Destination -Recurse -Force
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot 'Diagnose.cmd') -Destination $Destination -Force
    foreach ($folder in @('qemu','system')) { New-Item -ItemType Directory -Force -Path (Join-Path $Destination $folder) | Out-Null }
    Write-Host "Launcher UI + Rust core packaged at $Destination. Runtime/base image are separate."
} finally { Pop-Location }
