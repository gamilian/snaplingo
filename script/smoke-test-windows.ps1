param([string]$Installer)
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true') { throw 'Run this installer smoke test on an ephemeral GitHub Actions runner.' }
if (-not $Installer) {
    $installers = @(Get-ChildItem 'target/release/bundle/nsis' -Filter '*.exe')
    if ($installers.Count -ne 1) { throw 'Expected exactly one NSIS installer.' }
    $Installer = $installers[0].FullName
}
$Installer = (Resolve-Path $Installer).Path
$installDirectory = Join-Path $env:LOCALAPPDATA 'SnapLingo-release-smoke'
$appData = Join-Path $env:APPDATA 'com.snaplingo.app'
$marker = Join-Path $appData 'release-smoke-marker.txt'
$process = $null
try {
    # Exercise initial install and same-version reinstall; a real version upgrade
    # and interactive OCR/capture are covered by the release acceptance checklist.
    foreach ($attempt in 1..2) {
        $install = Start-Process -FilePath $Installer -ArgumentList "/S /D=$installDirectory" -Wait -PassThru
        if ($install.ExitCode -ne 0) { throw "Installer exited with $($install.ExitCode)" }
        if ($attempt -eq 2 -and -not (Test-Path $marker)) { throw 'Reinstall removed application data.' }
        $app = Join-Path $installDirectory 'snaplingo.exe'
        if (-not (Test-Path $app)) { throw "Installed executable missing: $app" }
        $process = Start-Process -FilePath $app -PassThru
        Start-Sleep -Seconds 8
        $process.Refresh()
        if ($process.HasExited) { throw "Installed SnapLingo exited with $($process.ExitCode)" }
        Stop-Process -Id $process.Id -Force
        $process.WaitForExit()
        $process = $null
        New-Item -ItemType Directory -Path $appData -Force | Out-Null
        Set-Content -Path $marker -Value 'Preserve user data on reinstall'
    }
    $uninstaller = Join-Path $installDirectory 'uninstall.exe'
    if (-not (Test-Path $uninstaller)) { throw 'Uninstaller missing.' }
    $uninstall = Start-Process -FilePath $uninstaller -ArgumentList "/S _?=$installDirectory" -Wait -PassThru
    if ($uninstall.ExitCode -ne 0) { throw "Uninstaller exited with $($uninstall.ExitCode)" }
    if (Test-Path (Join-Path $installDirectory 'snaplingo.exe')) { throw 'Uninstall did not remove the app.' }
    if (-not (Test-Path $marker)) { throw 'Uninstall unexpectedly removed user data.' }
} finally {
    if ($process -and -not $process.HasExited) { Stop-Process -Id $process.Id -Force }
    Remove-Item -Path $marker -Force -ErrorAction SilentlyContinue
}
