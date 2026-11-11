param(
    [ValidateSet('nsis', 'msi', 'portable')]
    [string]$InstallerType = 'nsis',
    [string]$Installer
)

$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true') { throw 'Run this installer smoke test on an ephemeral GitHub Actions runner.' }

if (-not $Installer) {
    $bundleDirectory = "target/release/bundle/$InstallerType"
    $extension = if ($InstallerType -eq 'msi') { '*.msi' } elseif ($InstallerType -eq 'portable') { '*-portable.zip' } else { '*.exe' }
    $installers = @(Get-ChildItem $bundleDirectory -Filter $extension)
    if ($installers.Count -ne 1) { throw "Expected exactly one $InstallerType installer." }
    $Installer = $installers[0].FullName
}
$Installer = (Resolve-Path $Installer).Path

$installDirectory = Join-Path $env:LOCALAPPDATA "SnapLingo-release-smoke-$InstallerType"
$appData = Join-Path $env:APPDATA 'com.snaplingo.app'
$marker = Join-Path $appData "release-smoke-marker-$InstallerType.txt"
$script:smokeProcess = $null
$script:msiProduct = $null

function Assert-AppStarts([string]$AppPath) {
    if (-not (Test-Path $AppPath)) { throw "Installed executable missing: $AppPath" }
    $applicationDirectory = Split-Path $AppPath -Parent
    foreach ($relative in @('vcruntime140.dll', 'vcruntime140_1.dll', 'msvcp140.dll', 'msvcp140_1.dll', 'ocr\small_det.mnn', 'ocr\small_rec.mnn', 'ocr\small_keys.txt')) {
        if (-not (Test-Path (Join-Path $applicationDirectory $relative))) { throw "Installed OCR dependency missing: $relative" }
    }
    $resultPath = Join-Path $env:TEMP "snaplingo-small-ocr-$PID.json"
    $fixture = Join-Path $PWD 'src-tauri/tests/fixtures/ocr-medium-small.png'
    try {
        $worker = Start-Process -FilePath $AppPath -ArgumentList @(
            '--ocr-small-worker',
            ('"' + (Join-Path $applicationDirectory 'ocr') + '"'),
            ('"' + $fixture + '"'),
            ('"' + $resultPath + '"')
        ) -Wait -PassThru
        if ($worker.ExitCode -ne 0) { throw "Installed small OCR worker exited with $($worker.ExitCode)" }
        $result = Get-Content -Path $resultPath -Raw | ConvertFrom-Json
        if (-not $result.Ok.text -or -not $result.Ok.lines.Count) { throw 'Installed small OCR worker returned no text' }
    } finally {
        Remove-Item -Path $resultPath -Force -ErrorAction SilentlyContinue
    }
    $script:smokeProcess = Start-Process -FilePath $AppPath -PassThru
    try {
        Start-Sleep -Seconds 8
        $script:smokeProcess.Refresh()
        if ($script:smokeProcess.HasExited) { throw "Installed SnapLingo exited with $($script:smokeProcess.ExitCode)" }
    } finally {
        if ($script:smokeProcess -and -not $script:smokeProcess.HasExited) {
            Stop-Process -Id $script:smokeProcess.Id -Force
            $script:smokeProcess.WaitForExit()
        }
        $script:smokeProcess = $null
    }
}

function Find-MsiProduct {
    $registryPaths = @(
        'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
        'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
        'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*'
    )
    foreach ($registryPath in $registryPaths) {
        $entries = @(Get-ItemProperty -Path $registryPath -ErrorAction SilentlyContinue)
        $entry = $entries |
            Where-Object { [string]$_.DisplayName -like 'SnapLingo*' } |
            Select-Object -First 1
        if ($entry) { return $entry }
    }
    throw 'SnapLingo MSI product registration was not found.'
}

function Get-MsiInstallDirectory($Product) {
    $directory = ([string]$Product.InstallLocation).Trim().TrimEnd('\')
    if ($directory) { return $directory }

    $icon = ([string]$Product.DisplayIcon).Trim().Trim('"') -replace ',\d+$', ''
    if ($icon) { return Split-Path $icon -Parent }
    throw 'SnapLingo MSI installation directory was not registered.'
}

function Install-Msi {
    $log = Join-Path $env:TEMP "snaplingo-msi-smoke-$PID.log"
    $quotedInstaller = '"' + $Installer + '"'
    $quotedLog = '"' + $log + '"'
    $install = Start-Process -FilePath 'msiexec.exe' -ArgumentList @(
        '/i', $quotedInstaller, '/qn', '/norestart', '/l*v', $quotedLog
    ) -Wait -PassThru
    if ($install.ExitCode -ne 0) { throw "MSI installer exited with $($install.ExitCode). Log: $log" }
    return Find-MsiProduct
}

function Uninstall-Msi($Product) {
    $productCode = [string]$Product.PSChildName
    if (-not $productCode) { throw 'SnapLingo MSI product code was not registered.' }
    $uninstall = Start-Process -FilePath 'msiexec.exe' -ArgumentList @(
        '/x', $productCode, '/qn', '/norestart'
    ) -Wait -PassThru
    if ($uninstall.ExitCode -ne 0) { throw "MSI uninstaller exited with $($uninstall.ExitCode)" }
}

try {
    if ($InstallerType -eq 'portable') {
        Expand-Archive -LiteralPath $Installer -DestinationPath $installDirectory -Force
        $executables = @(Get-ChildItem $installDirectory -Recurse -Filter 'snaplingo.exe')
        if ($executables.Count -ne 1) { throw 'Expected exactly one portable executable.' }
        Assert-AppStarts $executables[0].FullName
        Assert-AppStarts $executables[0].FullName
    } elseif ($InstallerType -eq 'nsis') {
        foreach ($attempt in 1..2) {
            $install = Start-Process -FilePath $Installer -ArgumentList "/S /D=$installDirectory" -Wait -PassThru
            if ($install.ExitCode -ne 0) { throw "Installer exited with $($install.ExitCode)" }
            $app = Join-Path $installDirectory 'snaplingo.exe'
            Assert-AppStarts $app
            if ($attempt -eq 2 -and -not (Test-Path $marker)) { throw 'Reinstall removed application data.' }
            New-Item -ItemType Directory -Path $appData -Force | Out-Null
            Set-Content -Path $marker -Value 'Preserve user data on reinstall'
        }
        $uninstaller = Join-Path $installDirectory 'uninstall.exe'
        if (-not (Test-Path $uninstaller)) { throw 'Uninstaller missing.' }
        $uninstall = Start-Process -FilePath $uninstaller -ArgumentList "/S _?=$installDirectory" -Wait -PassThru
        if ($uninstall.ExitCode -ne 0) { throw "Uninstaller exited with $($uninstall.ExitCode)" }
        if (Test-Path (Join-Path $installDirectory 'snaplingo.exe')) { throw 'Uninstall did not remove the app.' }
    } else {
        $script:msiProduct = Install-Msi
        $app = Join-Path (Get-MsiInstallDirectory $script:msiProduct) 'snaplingo.exe'
        Assert-AppStarts $app
        New-Item -ItemType Directory -Path $appData -Force | Out-Null
        Set-Content -Path $marker -Value 'Preserve user data on MSI reinstall'

        Uninstall-Msi $script:msiProduct
        $script:msiProduct = $null
        if (Test-Path $app) { throw 'MSI uninstall did not remove the app.' }
        if (-not (Test-Path $marker)) { throw 'MSI uninstall unexpectedly removed user data.' }

        $script:msiProduct = Install-Msi
        $app = Join-Path (Get-MsiInstallDirectory $script:msiProduct) 'snaplingo.exe'
        if (-not (Test-Path $marker)) { throw 'MSI reinstall removed application data.' }
        Assert-AppStarts $app
    }
} finally {
    if ($script:smokeProcess -and -not $script:smokeProcess.HasExited) { Stop-Process -Id $script:smokeProcess.Id -Force }
    if ($script:msiProduct) {
        try { Uninstall-Msi $script:msiProduct } catch { Write-Warning "MSI cleanup failed: $_" }
    }
    Remove-Item -Path $marker -Force -ErrorAction SilentlyContinue
    if ($InstallerType -eq 'portable') { Remove-Item -Path $installDirectory -Recurse -Force -ErrorAction SilentlyContinue }
}
