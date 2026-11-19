#Requires -Version 7.2
[CmdletBinding()]
param(
    [string]$OutputDirectory = (Join-Path $PSScriptRoot 'g6-hardware-out'),
    [string]$DeviceMatch = 'G6',
    [string]$InputMatch = '',
    [double]$SecondsPerBand = 15,
    [int]$WaitSeconds = 0,
    [switch]$SkipRecord,
    [switch]$SkipUsb,
    [switch]$SkipSanity,
    [switch]$PreflightOnly
)

$ErrorActionPreference = 'Stop'
$python = Join-Path $PSScriptRoot '..\audio-env\Scripts\python.exe'
$measure = Join-Path $PSScriptRoot 'measure-scout.py'
$usb = Join-Path $PSScriptRoot 'usbpcap-single-band.py'
$usbpcap = 'C:\Program Files\USBPcap\USBPcapCMD.exe'
$tshark = 'C:\Program Files\Wireshark\tshark.exe'

Write-Host '== Preflight =='
function Find-G6 {
    Get-CimInstance Win32_PnPEntity -ErrorAction SilentlyContinue |
        Where-Object { $_.DeviceID -match 'VID_041E&PID_3256' -or $_.Name -match 'Sound Blaster G6' } |
        Select-Object -First 1
}
$g6 = Find-G6
if (-not $g6 -and $WaitSeconds -gt 0) {
    $deadline = (Get-Date).AddSeconds($WaitSeconds)
    Write-Host "Waiting up to $WaitSeconds s for the G6 (plug/power it on now)..."
    while ((Get-Date) -lt $deadline -and -not $g6) {
        Start-Sleep -Seconds 5
        $g6 = Find-G6
        if (-not $g6) {
            Write-Host ("  still not present at {0}" -f (Get-Date).ToString('HH:mm:ss'))
        }
    }
}
if (-not $g6) {
    $keys = @(Get-ChildItem 'HKLM:\SYSTEM\CurrentControlSet\Enum\USB' -ErrorAction SilentlyContinue |
        Where-Object { $_.PSChildName -match 'VID_041E' } | ForEach-Object { $_.PSChildName })
    if ($keys.Count -gt 0) {
        Write-Host ("Driver registry keys exist but the device is not enumerated: {0}" -f ($keys -join ', ')) -ForegroundColor Yellow
    }
    Write-Host 'G6 not connected (VID_041E/PID_3256). Connect/power the device and rerun.' -ForegroundColor Red
    exit 2
}
Write-Host "G6: $($g6.Name) [$($g6.DeviceID)]"
if (-not (Test-Path -LiteralPath $python)) {
    Write-Host "Python audio environment missing: $python" -ForegroundColor Red
    exit 2
}
if (-not (Test-Path -LiteralPath $usbpcap)) {
    Write-Host "USBPcapCMD missing: $usbpcap" -ForegroundColor Red
    exit 2
}
if (-not (Test-Path -LiteralPath $tshark)) {
    Write-Host "tshark missing: $tshark" -ForegroundColor Red
    exit 2
}
$service = Get-Service -Name USBPcap -ErrorAction SilentlyContinue
if (-not $service -or $service.Status -ne 'Running') {
    Write-Host 'USBPcap service is not running; start it from an elevated prompt.' -ForegroundColor Red
    exit 2
}
$probe = @'
import sys
import sounddevice as sd
match = sys.argv[1].lower()
outputs = [d for d in sd.query_devices() if d["max_output_channels"] > 0 and match in d["name"].lower()]
inputs = [d for d in sd.query_devices() if d["max_input_channels"] > 0]
print("outputs:" + " | ".join(d["name"] for d in outputs))
print("inputs:" + " | ".join(d["name"] for d in inputs[:8]))
raise SystemExit(0 if outputs else 1)
'@
& $python -I -c $probe $DeviceMatch
if ($LASTEXITCODE -ne 0) {
    Write-Host "No audio output endpoint matches '$DeviceMatch'." -ForegroundColor Red
    exit 2
}
if (-not $InputMatch) {
    Write-Host 'No -InputMatch supplied: the -Record step will need a loopback/physical input name.' -ForegroundColor Yellow
}
if ($PreflightOnly) {
    Write-Host 'PASS: preflight only'
    exit 0
}

New-Item -ItemType Directory -Force -Path $OutputDirectory | Out-Null
$results = [ordered]@{
    preflight = @{ device = $g6.Name; instance_id = $g6.DeviceID; timestamp = (Get-Date).ToString('o') }
}

if (-not $SkipRecord) {
    Write-Host '== Scout sweep: OFF then ON ==' -ForegroundColor Cyan
    if (-not $InputMatch) {
        Write-Host 'Refusing to record without -InputMatch (use a physical loopback or loopback input).' -ForegroundColor Red
        exit 2
    }
    & $python -I $measure --record --device-match $DeviceMatch --input-match $InputMatch `
        --output-dir $OutputDirectory --label scout
    if ($LASTEXITCODE -ne 0) { Write-Host 'Scout record/analyze failed.' -ForegroundColor Red; exit 3 }
    $results.sweep = Get-Content -Raw -LiteralPath (Join-Path $OutputDirectory 'scout-response.json') | ConvertFrom-Json
}

if (-not $SkipSanity) {
    Write-Host '== Sanity: Flat vs 31 Hz +6 dB ==' -ForegroundColor Cyan
    if (-not $InputMatch) {
        Write-Host 'Sanity check also needs -InputMatch.' -ForegroundColor Red
        exit 2
    }
    & $python -I $measure --record --device-match $DeviceMatch --input-match $InputMatch `
        --output-dir $OutputDirectory --label sanity `
        --prompt-off 'Set the graphic EQ to Flat (all bands 0 dB), then press Enter...' `
        --prompt-on 'Set only the 31 Hz band to +6 dB, then press Enter...'
    if ($LASTEXITCODE -ne 0) { Write-Host 'Sanity record/analyze failed.' -ForegroundColor Red; exit 3 }
    $sanity = Get-Content -Raw -LiteralPath (Join-Path $OutputDirectory 'sanity-response.json') | ConvertFrom-Json
    $band = $sanity.bands | Where-Object { $_.hz -lt 40 } | Select-Object -First 1
    $delta = [double]$band.delta_db
    $ok = [math]::Abs($delta - 6.0) -le 0.8
    $results.sanity = @{ band_hz = $band.hz; delta_db = $delta; expected_db = 6.0; passed = $ok }
    if (-not $ok) {
        Write-Host "Sanity check failed: 31 Hz delta $delta dB, expected 6 dB +/- 0.8." -ForegroundColor Yellow
    } else {
        Write-Host "Sanity check PASS: 31 Hz delta $delta dB" -ForegroundColor Green
    }
}

if (-not $SkipUsb) {
    Write-Host '== USBPcap: one band per capture ==' -ForegroundColor Cyan
    $bands = @(31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000)
    $mapping = @()
    foreach ($band in $bands) {
        $pcap = Join-Path $OutputDirectory ("band-{0}.pcap" -f $band)
        $report = Join-Path $OutputDirectory ("band-{0}.json" -f $band)
        Write-Host ("Capture for {0} Hz: when it starts, set ONLY this band to +3 dB; reset it afterwards." -f $band) -ForegroundColor Cyan
        & $python -I $usb --capture --seconds $SecondsPerBand --output $pcap --report $report
        if ($LASTEXITCODE -ne 0) {
            Write-Host ("Capture/parse failed for {0} Hz." -f $band) -ForegroundColor Yellow
            $mapping += [pscustomobject]@{ band_hz = $band; id = $null; before = $null; after = $null; note = 'capture-failed' }
        } else {
            $parsed = Get-Content -Raw -LiteralPath $report | ConvertFrom-Json
            $changed = @($parsed.diff.changed)
            if ($changed.Count -eq 1) {
                $mapping += [pscustomobject]@{ band_hz = $band; id = $changed[0].id;
                    before = $changed[0].before; after = $changed[0].after; note = 'ok' }
                Write-Host ("mapped {0} Hz -> id {1} ({2} -> {3})" -f $band, $changed[0].id, $changed[0].before, $changed[0].after) -ForegroundColor Green
            } else {
                Write-Host ("{0} Hz: expected one changed id, found {1}." -f $band, $changed.Count) -ForegroundColor Yellow
                $mapping += [pscustomobject]@{ band_hz = $band; id = $null; before = $null; after = $null; note = 'ambiguous' }
            }
        }
        Read-Host 'Reset the band to 0 dB, then press Enter'
    }
    $results.id_map = $mapping
    $mapping | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $OutputDirectory 'g6-scout-id-map.json')
}

$results | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $OutputDirectory 'g6-hardware-report.json')
Write-Host ("written: {0}" -f (Join-Path $OutputDirectory 'g6-hardware-report.json'))
exit 0
