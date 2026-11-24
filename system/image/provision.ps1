param(
    [Parameter(Mandatory=$true)][string]$Qemu,
    [Parameter(Mandatory=$true)][string]$Disk,
    [Parameter(Mandatory=$true)][string]$Seed,
    [ValidateSet('tcg','whpx')][string]$Accel = 'tcg'
)
$ErrorActionPreference = 'Stop'
# Dedicated build VM only. Disk must be a QCOW2 file, never a physical drive.
$diskFile = Get-Item -LiteralPath $Disk
if ($diskFile.Extension -ne '.qcow2') { throw 'Expected a QCOW2 file' }
$qemuFile = Get-Item -LiteralPath $Qemu
$seedFile = Get-Item -LiteralPath $Seed
$firmware = [IO.Path]::GetRelativePath($diskFile.DirectoryName, (Join-Path $qemuFile.DirectoryName "share"))
$seedRelative = [IO.Path]::GetRelativePath($diskFile.DirectoryName, $seedFile.FullName)
Push-Location $diskFile.DirectoryName
try {
    & $qemuFile.FullName -L $firmware -machine q35 -accel $Accel -m 4096 -smp 2 -drive "file=$($diskFile.Name.Replace(',', ',,')),format=qcow2,if=virtio" -cdrom $seedRelative -netdev user,id=n0 -device e1000,netdev=n0 -vga virtio -device virtio-rng-pci
    if ($LASTEXITCODE -ne 0) { throw 'Provisioning VM failed' }
} finally { Pop-Location }
