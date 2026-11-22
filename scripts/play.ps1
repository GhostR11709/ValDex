# Starts Vivi if it is not already running, then opens the player window.
param(
  [switch]$Quit
)

$ErrorActionPreference = "Stop"
$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$uri = "http://127.0.0.1:4731"
$lock = Join-Path $env:TEMP "vivi-starting.lock"
Set-Location $root

function Test-ViviUp {
  try {
    $response = Invoke-WebRequest -Uri $uri -UseBasicParsing -TimeoutSec 8
    return ($response.StatusCode -ge 200 -and $response.StatusCode -lt 500)
  } catch {
    return $false
  }
}

function Stop-Vivi {
  $listen = @(Get-NetTCPConnection -LocalPort 4731 -State Listen -ErrorAction SilentlyContinue)
  $pids = @($listen | Select-Object -ExpandProperty OwningProcess -Unique)
  foreach ($procId in $pids) {
    if ($procId -and $procId -ne 0) {
      & taskkill.exe /PID $procId /T /F | Out-Null
    }
  }
  if (Test-Path $lock) {
    Remove-Item $lock -Force -ErrorAction SilentlyContinue
  }
}

function Open-Vivi {
  $browsers = @(
    "$env:ProgramFiles\Microsoft\Edge\Application\msedge.exe",
    "${env:ProgramFiles(x86)}\Microsoft\Edge\Application\msedge.exe",
    "$env:ProgramFiles\Google\Chrome\Application\chrome.exe",
    "${env:ProgramFiles(x86)}\Google\Chrome\Application\chrome.exe"
  )
  foreach ($browser in $browsers) {
    if (Test-Path $browser) {
      Start-Process -FilePath $browser -ArgumentList @("--app=$uri", "--window-size=1280,800")
      return
    }
  }
  Start-Process $uri
}

function Ensure-DesktopShortcut {
  $desktop = [Environment]::GetFolderPath("Desktop")
  if (-not $desktop) {
    return
  }
  $shortcutPath = Join-Path $desktop "Vivi.lnk"
  $target = Join-Path $root "Play Vivi.bat"
  $icon = Join-Path $root "vivi.ico"
  $shell = New-Object -ComObject WScript.Shell
  $shortcut = $shell.CreateShortcut($shortcutPath)
  $shortcut.TargetPath = $target
  $shortcut.WorkingDirectory = $root
  $shortcut.WindowStyle = 1
  $shortcut.Description = "Open Vivi"
  if (Test-Path $icon) {
    $shortcut.IconLocation = $icon
  }
  $shortcut.Save()
}

if ($Quit) {
  Stop-Vivi
  Write-Host "Vivi is closed."
  Start-Sleep -Seconds 1
  exit 0
}

if (-not (Get-Command node -ErrorAction SilentlyContinue)) {
  Write-Host "Node.js is not installed."
  Write-Host "Install it from https://nodejs.org/ and then open Vivi again."
  Read-Host "Press Enter to close"
  exit 1
}

if (-not (Test-Path (Join-Path $root "node_modules"))) {
  Write-Host "First launch: installing Vivi. This only happens once."
  & npm install
  if ($LASTEXITCODE -ne 0) {
    Write-Host "Install failed."
    Read-Host "Press Enter to close"
    exit 1
  }
}

Ensure-DesktopShortcut

if (Test-ViviUp) {
  Open-Vivi
  exit 0
}

if (Test-Path $lock) {
  $age = (Get-Date) - (Get-Item $lock).LastWriteTime
  if ($age.TotalMinutes -gt 3) {
    Remove-Item $lock -Force -ErrorAction SilentlyContinue
  }
}

$weStarted = $false
if (-not (Test-Path $lock)) {
  try {
    $handle = [System.IO.File]::Open($lock, "CreateNew", "Write", "None")
    $handle.Close()
    $weStarted = $true
  } catch {
    $weStarted = $false
  }
}

if ($weStarted) {
  Write-Host "Starting Vivi..."
  $log = Join-Path $env:TEMP "vivi-dev.log"
  $server = Join-Path $PSScriptRoot "dev-server.bat"
  $proc = Start-Process -FilePath "$env:SystemRoot\System32\cmd.exe" -ArgumentList "/d /c `"$server`"" -WorkingDirectory $root -PassThru -WindowStyle Hidden

  $deadline = (Get-Date).AddSeconds(120)
  while (-not (Test-ViviUp)) {
    if ($proc.HasExited -or (Get-Date) -gt $deadline) {
      Write-Host "Vivi did not start."
      if (Test-Path $log) {
        Get-Content $log -Tail 30
      }
      Remove-Item $lock -Force -ErrorAction SilentlyContinue
      if (-not $proc.HasExited) {
        & taskkill.exe /PID $proc.Id /T /F | Out-Null
      }
      Read-Host "Press Enter to close"
      exit 1
    }
    Start-Sleep -Milliseconds 500
  }
  Remove-Item $lock -Force -ErrorAction SilentlyContinue
} else {
  Write-Host "Vivi is already starting..."
  $deadline = (Get-Date).AddSeconds(120)
  while (-not (Test-ViviUp)) {
    if ((Get-Date) -gt $deadline) {
      Write-Host "Vivi did not start."
      Read-Host "Press Enter to close"
      exit 1
    }
    Start-Sleep -Milliseconds 500
  }
}

Open-Vivi
exit 0
