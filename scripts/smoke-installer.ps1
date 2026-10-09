# Smoke test for the Windows installer (ADR-0068). Meant to run inside Windows Sandbox, a
# clean machine with no Rust, Node or Visual C++ runtime; see `scripts/smoke-installer.wsb`.
#
#   install silently -> files present -> start the daemon on the default port -> project scan
#   -> plugin scan (a fake plugin proves the worker is found and runs) -> stop -> uninstall
#
# -BinDir skips the install and uninstall and runs the middle against a folder holding
# seula.exe and vst-meta.exe, to debug this script on a development machine. Use a -Port
# that is not your running daemon's.
#
# Prints PASS/FAIL per check, writes the same to <OutDir>\smoke-result.txt, and exits
# non-zero if any check failed.

param(
    [string]$InstallerDir = 'C:\in',
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\Seula",
    [string]$OutDir = 'C:\out',
    [string]$BinDir,
    [int]$Port = 50052
)

$ErrorActionPreference = 'Stop'
New-Item -ItemType Directory -Force $OutDir | Out-Null
$resultFile = Join-Path $OutDir 'smoke-result.txt'
Set-Content $resultFile "smoke run $(Get-Date -Format s)"
$script:failed = 0

function Check([string]$name, [bool]$ok, [string]$detail = '') {
    $line = "$(if ($ok) { 'PASS' } else { 'FAIL' })  $name$(if ($detail) { "  ($detail)" })"
    Write-Host $line
    Add-Content $resultFile $line
    if (-not $ok) { $script:failed++ }
}

function Note([string]$text) { Write-Host $text; Add-Content $resultFile $text }

$url = "http://127.0.0.1:$Port"
$work = Join-Path $env:TEMP "seula-smoke-$([guid]::NewGuid().ToString('N').Substring(0, 8))"
$projects = Join-Path $work 'projects'
$plugins = Join-Path $work 'plugins'
$data = Join-Path $work 'data'
New-Item -ItemType Directory -Force $projects, $plugins, $data | Out-Null
$daemon = $null

try {
    # 1. Install
    if ($BinDir) {
        $bin = $BinDir
    } else {
        $installer = Get-ChildItem $InstallerDir -Filter '*-setup.exe' | Select-Object -First 1
        Check 'installer found' ($null -ne $installer) $InstallerDir
        $p = Start-Process $installer.FullName -ArgumentList '/S', "/D=$InstallDir" -Wait -PassThru
        Check 'silent install exits 0' ($p.ExitCode -eq 0) "exit $($p.ExitCode)"
        $bin = $InstallDir
        foreach ($f in 'seula-shell.exe', 'seula.exe', 'vst-meta.exe', 'LICENSE', 'LICENSE-EXCEPTIONS.md') {
            Check "installed: $f" (Test-Path (Join-Path $bin $f))
        }
        Check 'uninstaller installed' (Test-Path (Join-Path $bin 'uninstall.exe'))
    }

    # 2. Fixtures: two minimal projects (EnumEvent 201 is 4/4, how the parser reads the time
    #    signature), and a fake plugin that cannot load
    $xml = '<?xml version="1.0" encoding="UTF-8"?><Ableton MajorVersion="5" MinorVersion="12.0_12049" SchemaChangeCount="7" Creator="Ableton Live 12.0"><LiveSet><Tempo><Manual Value="128.0"/></Tempo><EnumEvent Id="0" Value="201"/></LiveSet></Ableton>'
    foreach ($n in 'Alpha', 'Beta') {
        $fs = [IO.File]::Create((Join-Path $projects "$n.als"))
        $gz = New-Object IO.Compression.GZipStream($fs, [IO.Compression.CompressionMode]::Compress)
        $bytes = [Text.Encoding]::UTF8.GetBytes($xml)
        $gz.Write($bytes, 0, $bytes.Length); $gz.Dispose(); $fs.Dispose()
    }
    [IO.File]::WriteAllBytes((Join-Path $plugins 'Fake Synth.dll'), [byte[]]@())

    # 3. Config: the default port on purpose, so the frontend's default URL is exercised
    $q = { param($p) "'" + ($p -replace '\\', '/') + "'" }
    @"
paths = [$(& $q $projects)]
database_path = $(& $q (Join-Path $data 'seula.db'))
media_storage_dir = $(& $q (Join-Path $data 'media'))
http_port = $Port
log_level = 'info'
vst_search_paths = [$(& $q $plugins)]
"@ | Set-Content (Join-Path $work 'config.toml') -Encoding utf8

    # 4. Start the daemon
    Get-ChildItem env:SEULA_* -ErrorAction SilentlyContinue | Remove-Item
    $daemon = Start-Process (Join-Path $bin 'seula.exe') -ArgumentList '--config', (Join-Path $work 'config.toml'), '--server' `
        -PassThru -RedirectStandardOutput (Join-Path $OutDir 'daemon.out.log') -RedirectStandardError (Join-Path $OutDir 'daemon.err.log')
    $up = $false
    for ($i = 0; $i -lt 150 -and -not $up; $i++) {
        if ($daemon.HasExited) { break }
        try { $null = Invoke-WebRequest "$url/health" -UseBasicParsing -TimeoutSec 1; $up = $true } catch { Start-Sleep -Milliseconds 200 }
    }
    Check 'daemon answers /health' $up $(if ($daemon.HasExited) { "exited $($daemon.ExitCode)" } else { $url })
    if (-not $up) { throw 'daemon never came up' }

    # 5. Project scan (the response is an SSE stream that ends when the scan does)
    $scan = Invoke-WebRequest "$url/api/v1/system/scan" -Method Post -UseBasicParsing -TimeoutSec 120
    Add-Content (Join-Path $OutDir 'project-scan.sse.txt') $scan.Content
    $list = Invoke-RestMethod "$url/api/v1/projects?limit=100"
    Check 'project scan finds both fixtures' ($list.total_count -eq 2) "total_count=$($list.total_count)"

    # 6. Plugin scan: the fake plugin must be reported, not WorkerNotFound
    $ps = Invoke-WebRequest "$url/api/v1/plugins/scan?mode=all" -Method Post -UseBasicParsing -TimeoutSec 180
    Add-Content (Join-Path $OutDir 'plugin-scan.sse.txt') $ps.Content
    Check 'plugin scan: worker was found' ($ps.Content -notmatch 'Could not find the plugin scanner')
    Check 'plugin scan: worker ran and reported the fake plugin' ($ps.Content -match 'Fake Synth') 'see plugin-scan.sse.txt'
    Check 'daemon still alive after the worker ran' (-not $daemon.HasExited)

    # 7. Log file (item F; recorded, not required yet)
    $logs = Get-ChildItem -Recurse (Join-Path $env:APPDATA 'Seula') -Filter '*.log*' -ErrorAction SilentlyContinue
    Note "info  log files under %APPDATA%\Seula: $(@($logs).Count)"
} catch {
    Check 'script ran to the end' $false $_.Exception.Message
} finally {
    if ($daemon -and -not $daemon.HasExited) { Stop-Process -Id $daemon.Id -Force }
    if ($daemon) { Start-Sleep -Milliseconds 500 }
}

# 8. Uninstall: the files go, the user's data stays
if (-not $BinDir -and (Test-Path (Join-Path $InstallDir 'uninstall.exe'))) {
    $u = Start-Process (Join-Path $InstallDir 'uninstall.exe') -ArgumentList '/S', "_?=$InstallDir" -Wait -PassThru
    Check 'silent uninstall exits 0' ($u.ExitCode -eq 0) "exit $($u.ExitCode)"
    Check 'uninstall removed seula.exe' (-not (Test-Path (Join-Path $InstallDir 'seula.exe')))
    Check 'uninstall left user data alone' (Test-Path (Join-Path $data 'seula.db'))
}

# 9. Upgrade and uninstall while the daemon is running. The shell leaves the daemon
#    running after its window closes (ADR-0069), so this is the normal state at both.
if (-not $BinDir) {
    function Start-Daemon {
        Start-Process (Join-Path $InstallDir 'seula.exe') -ArgumentList '--config', (Join-Path $work 'config.toml'), '--server' -PassThru `
            -RedirectStandardOutput (Join-Path $OutDir 'daemon2.out.log') -RedirectStandardError (Join-Path $OutDir 'daemon2.err.log')
    }
    function Wait-Health {
        for ($i = 0; $i -lt 75; $i++) {
            try { $null = Invoke-WebRequest "$url/health" -UseBasicParsing -TimeoutSec 1; return $true } catch { Start-Sleep -Milliseconds 200 }
        }
        $false
    }
    $installer = Get-ChildItem $InstallerDir -Filter '*-setup.exe' | Select-Object -First 1
    $null = Start-Process $installer.FullName -ArgumentList '/S', "/D=$InstallDir" -Wait -PassThru
    $d = Start-Daemon
    Check 'upgrade test: daemon running' (Wait-Health)
    $up = Start-Process $installer.FullName -ArgumentList '/S', "/D=$InstallDir" -Wait -PassThru
    Check 'install over a running daemon exits 0' ($up.ExitCode -eq 0) "exit $($up.ExitCode)"
    Check 'install stopped the old daemon' ($null -eq (Get-Process -Id $d.Id -ErrorAction SilentlyContinue))
    Check 'install replaced seula.exe' (Test-Path (Join-Path $InstallDir 'seula.exe'))

    $d = Start-Daemon
    Check 'uninstall test: daemon running' (Wait-Health)
    $un = Start-Process (Join-Path $InstallDir 'uninstall.exe') -ArgumentList '/S', "_?=$InstallDir" -Wait -PassThru
    Check 'uninstall with a running daemon exits 0' ($un.ExitCode -eq 0) "exit $($un.ExitCode)"
    Check 'uninstall stopped the daemon' ($null -eq (Get-Process -Id $d.Id -ErrorAction SilentlyContinue))
    Check 'uninstall removed seula.exe' (-not (Test-Path (Join-Path $InstallDir 'seula.exe')))
    Check 'uninstall removed vst-meta.exe' (-not (Test-Path (Join-Path $InstallDir 'vst-meta.exe')))
    Check 'user data survived both' (Test-Path (Join-Path $data 'seula.db'))
}

Note "$(if ($script:failed) { "$script:failed CHECK(S) FAILED" } else { 'ALL CHECKS PASSED' })"
if (-not $BinDir) { Remove-Item -Recurse -Force $work -ErrorAction SilentlyContinue }
exit $(if ($script:failed) { 1 } else { 0 })
