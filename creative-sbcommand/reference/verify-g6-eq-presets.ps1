#Requires -Version 7.2
[CmdletBinding()]
param([string]$SourceDirectory)

$ErrorActionPreference = 'Stop'
$snapshotPath = Join-Path $PSScriptRoot 'g6-eq-presets-current.json'
$snapshot = Get-Content -Raw -LiteralPath $snapshotPath | ConvertFrom-Json -AsHashtable
$presets = $snapshot.eqPresets
$bands = @(31,62,125,250,500,1000,2000,4000,8000,16000)
$modes = @('Speaker', 'Headphone')
$culture = [cultureinfo]::InvariantCulture
if ($snapshot.product -cne 'G6' -or $presets.Count -ne 43) {
    throw 'Expected 43 G6 presets in the delivery snapshot.'
}
if ($SourceDirectory) {
    $sourceFiles = @(Get-ChildItem -LiteralPath $SourceDirectory -File -Filter '*.json')
    if ($sourceFiles.Count -ne 43) { throw "Expected 43 source files, found $($sourceFiles.Count)." }
}

$sourceNames = [Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
$expected = @(foreach ($entry in $presets.GetEnumerator()) {
    $p = $entry.Value
    if ($p.id -cne $entry.Key -or $p.id -cnotmatch '^[A-Za-z0-9]+$' -or
        $p.label_zh -cnotmatch '[\u4e00-\u9fff]' -or
        $p.source_file -cnotmatch '^[A-Za-z0-9_-]+\.json$' -or
        $p.source_sha256 -cnotmatch '^[a-f0-9]{64}$' -or
        -not $sourceNames.Add($p.source_file) -or $p.settings.Count -ne 2) {
        throw "Invalid snapshot entry: $($entry.Key)."
    }
    if ($SourceDirectory) {
        $path = Join-Path $SourceDirectory $p.source_file
        $bytes = [IO.File]::ReadAllBytes($path)
        $hash = [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes))
        if ($hash -ine $p.source_sha256) { throw "Source hash mismatch: $($p.source_file)." }
        $original = [Text.Encoding]::UTF8.GetString($bytes).TrimStart([char]0xfeff) | ConvertFrom-Json
        if ($original.Product -cne 'G6' -or $original.Id -cne $p.id -or
            $original.Order -ne $p.order -or $original.Type -ne $p.type -or
            $original.SpeakerMethod -ne $p.speaker_method -or $original.Settings.Count -ne 2) {
            throw "Source metadata mismatch: $($p.id)."
        }
    }
    foreach ($mode in $modes) {
        $s = $p.settings[$mode]
        if ($null -eq $s.preamp -or ($s.freqs -join ',') -ne ($bands -join ',') -or
            $s.values.Count -ne $bands.Count -or $s.values -contains $null) {
            throw "Invalid band layout: $($p.id) $mode."
        }
        if ($SourceDirectory) {
            $raw = @($original.Settings | Where-Object Type -CEQ $mode)
            if ($raw.Count -ne 1 -or $raw[0].Unit -cne 'dB' -or $raw[0].PreAmp -ne $s.preamp -or
                ($raw[0].Bands.Frequency -join ',') -ne ($s.freqs -join ',') -or
                ($raw[0].Bands.Value -join ',') -ne ($s.values -join ',')) {
                throw "Source settings mismatch: $($p.id) $mode."
            }
        }
        $fields = @($p.id, $p.order, $p.type, $p.speaker_method, $mode,
            ([single]$s.preamp).ToString('G9', $culture))
        for ($i = 0; $i -lt $bands.Count; ++$i) {
            $fields += "$($s.freqs[$i]):$(([single]$s.values[$i]).ToString('G9', $culture))"
        }
        $fields -join '|'
    }
})
$expected = @($expected | Sort-Object -CaseSensitive)

$vcvars = ''
if (-not (Get-Command cl.exe -ErrorAction SilentlyContinue)) {
    $installer = Join-Path ([Environment]::GetEnvironmentVariable('ProgramFiles(x86)')) 'Microsoft Visual Studio/Installer'
    $vswhere = Join-Path $installer 'vswhere.exe'
    if (-not (Test-Path -LiteralPath $vswhere)) {
        throw 'MSVC was not found. Install the C++ Build Tools or use a Developer PowerShell.'
    }
    $install = & $vswhere -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath |
        Select-Object -First 1
    if (-not $install) { throw 'No MSVC C++ Build Tools installation found.' }
    $vcvars = Join-Path $install 'VC/Auxiliary/Build/vcvars64.bat'
}

$work = Join-Path ([IO.Path]::GetTempPath()) ('g6-eq-acceptance-' + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $work | Out-Null
try {
    $sourceFiles = @('g6_eq_presets.c', 'g6_eq_presets.h', 'g6_eq_dsp.c', 'g6_eq_dsp.h',
                     'g6_eq_apo.c', 'g6_eq_apo.h')
    $inputs = @($sourceFiles | ForEach-Object { Join-Path $PSScriptRoot $_ })
    Copy-Item -LiteralPath $inputs -Destination $work
    $probe = @"
#include "g6_eq_dsp.h"
#include <math.h>
#include <stdio.h>

static int verify_dsp(void)
{
    const G6EqPreset *flat = g6_eq_find_preset("Flat");
    const G6EqPreset *fps = g6_eq_find_preset("FPS");
    G6EqChain chain;
    G6EqChannelState state[2] = {0};
    float samples[] = { 0.8f, 0.0f, 0.0f, 0.0f };

    if (flat == NULL || fps == NULL ||
        g6_eq_chain_design(&chain, &flat->speaker, 48000, 6.0) != G6_EQ_OK ||
        chain.filter_count != 0 || fabs(g6_eq_response_db(&chain, 1000) - 6.0) > 1e-10 ||
        g6_eq_process_interleaved(&chain, state, samples, 2, 2) != G6_EQ_OK ||
        samples[0] <= 1.59f || samples[0] >= 1.60f) {
        return 0;
    }

    g6_eq_reset_channel(&state[0]);
    g6_eq_reset_channel(&state[1]);
    samples[0] = 0.5f;
    return g6_eq_chain_design(&chain, &fps->headphone, 48000, 0.0) == G6_EQ_OK &&
           chain.filter_count > 0 &&
           g6_eq_process_interleaved(&chain, state, samples, 2, 2) == G6_EQ_OK &&
           samples[0] != 0.5f && samples[2] != 0.0f &&
           samples[1] == 0.0f && samples[3] == 0.0f;
}

static int verify_apo(void)
{
    const G6EqPreset *flat = g6_eq_find_preset("Flat");
    const G6EqPreset *fps = g6_eq_find_preset("FPS");
    G6EqApoChain chain;
    G6EqApoChannelState state[2] = {0};
    G6EqApoCoefficients coeff;
    float samples[] = { 0.8f, 0.0f, 0.0f, 0.0f };

    if (flat == NULL || fps == NULL ||
        g6_eq_apo_chain_design(&chain, &flat->speaker, 48000, 6.0) != G6_EQ_OK ||
        chain.filter_count != 0 ||
        g6_eq_apo_process_chain_interleaved(&chain, state, samples, 2, 2) != G6_EQ_OK ||
        samples[0] <= 1.59f || samples[0] >= 1.60f) {
        return 0;
    }

    g6_eq_apo_reset_channel_state(&state[0]);
    g6_eq_apo_reset_channel_state(&state[1]);
    samples[0] = 0.5f;
    samples[2] = 0.0f;
    if (g6_eq_apo_chain_design(&chain, &fps->headphone, 48000, 0.0) != G6_EQ_OK ||
        chain.filter_count == 0 ||
        g6_eq_apo_process_chain_interleaved(&chain, state, samples, 2, 2) != G6_EQ_OK ||
        samples[0] == 0.5f || samples[2] == 0.0f ||
        samples[1] != 0.0f || samples[3] != 0.0f) {
        return 0;
    }

    /* Exact peak slots, reference produced by an independent float32 emulation
       of 0x180087120. */
    if (g6_eq_apo_peak_db(6.0f, 48000.0f, 1000.0f, 2.0f, &coeff) != G6_EQ_APO_OK ||
        fabsf(coeff.c8 - 1.9508814f) > 1e-5f ||
        fabsf(coeff.c12 + 0.9677154f) > 1e-5f ||
        fabsf(coeff.c16 - 1.0160658f) > 1e-5f ||
        fabsf(coeff.c20 + 1.9508814f) > 1e-5f ||
        fabsf(coeff.c24 - 0.9516495f) > 1e-5f) {
        return 0;
    }
    return 1;
}

int main(void)
{
    if (g6_eq_preset_count != 43 || g6_eq_find_preset(NULL) != NULL ||
        g6_eq_find_preset("") != NULL || g6_eq_find_preset("UnknownPreset") != NULL ||
        g6_eq_find_preset("fps") != NULL) {
        return 1;
    }

    if (!verify_dsp()) {
        return 3;
    }

    if (!verify_apo()) {
        return 4;
    }

    for (size_t i = 0; i < g6_eq_preset_count; ++i) {
        const G6EqPreset *p = &g6_eq_presets[i];
        if (g6_eq_find_preset(p->id) != p) {
            return 2;
        }

        const G6EqSettings *settings[] = { &p->speaker, &p->headphone };
        const char *modes[] = { "Speaker", "Headphone" };
        for (size_t m = 0; m < 2; ++m) {
            printf("%s|%d|%d|%d|%s|%.9g", p->id, p->order, p->type,
                   p->speaker_method, modes[m], settings[m]->preamp_db);
            for (size_t b = 0; b < G6_EQ_BAND_COUNT; ++b) {
                printf("|%u:%.9g", g6_eq_band_hz[b], settings[m]->gain_db[b]);
            }
            putchar('\n');
        }
    }
    return 0;
}
"@
    Set-Content -LiteralPath (Join-Path $work 'probe.c') -Value $probe -Encoding ascii
    $batch = @(
        '@echo off'
        'setlocal'
        if ($vcvars) {
            ('set "PATH={0};%PATH%"' -f $installer)
            ('call "{0}" >nul || exit /b 1' -f $vcvars)
        }
        'cl.exe /nologo /TC /std:c11 /W4 /WX /c "%~dp0g6_eq_presets.c" /Fo"%~dp0g6_eq_presets.obj" || exit /b 1'
        'cl.exe /nologo /TC /std:c11 /W4 /WX /c "%~dp0g6_eq_apo.c" /Fo"%~dp0g6_eq_apo.obj" || exit /b 1'
        'cl.exe /nologo /TC /std:c11 /W4 /WX /c "%~dp0g6_eq_dsp.c" /Fo"%~dp0g6_eq_dsp.obj" || exit /b 1'
        'cl.exe /nologo /TC /std:c11 /W4 /WX /c "%~dp0probe.c" /Fo"%~dp0probe.obj" || exit /b 1'
        'cl.exe /nologo "%~dp0g6_eq_presets.obj" "%~dp0g6_eq_apo.obj" "%~dp0g6_eq_dsp.obj" "%~dp0probe.obj" /Fe"%~dp0g6_eq_check.exe"'
        'exit /b %errorlevel%'
    )
    $build = Join-Path $work 'build.cmd'
    Set-Content -LiteralPath $build -Value $batch -Encoding ascii
    & cmd.exe /d /c ('call "' + $build + '"')
    if ($LASTEXITCODE -ne 0) { throw "MSVC compilation failed (exit $LASTEXITCODE)." }

    $actual = @(& (Join-Path $work 'g6_eq_check.exe'))
    if ($LASTEXITCODE -ne 0) { throw "C lookup, count or DSP smoke check failed (exit $LASTEXITCODE)." }
    $actual = @($actual | Sort-Object -CaseSensitive)
    if ($actual.Count -ne 86) { throw "Expected 86 compiled mode records, found $($actual.Count)." }
    for ($i = 0; $i -lt $expected.Count; ++$i) {
        if ($actual[$i] -cne $expected[$i]) {
            throw "Compiled record mismatch. Expected: $($expected[$i]). Actual: $($actual[$i])."
        }
    }
    Get-FileHash -Algorithm SHA256 -LiteralPath ($inputs + $snapshotPath) |
        ForEach-Object { '{0}  {1}' -f $_.Hash, (Split-Path -Leaf $_.Path) }
    if ($SourceDirectory) { 'PASS: 43 source hashes and source parameter records match the snapshot.' }
    'PASS: MSVC C11 /W4 /WX, 43 IDs, 86 compiled mode records, case-sensitive lookup, and DSP smoke checks.'
} finally {
    foreach ($file in @('g6_eq_presets.c', 'g6_eq_presets.h', 'g6_eq_dsp.c', 'g6_eq_dsp.h',
                        'g6_eq_apo.c', 'g6_eq_apo.h', 'probe.c', 'build.cmd',
                        'g6_eq_presets.obj', 'g6_eq_apo.obj', 'g6_eq_dsp.obj', 'probe.obj',
                        'g6_eq_check.exe')) {
        $path = Join-Path $work $file
        if (Test-Path -LiteralPath $path) { Remove-Item -LiteralPath $path }
    }
    if (-not (Get-ChildItem -LiteralPath $work -Force)) {
        Remove-Item -LiteralPath $work -ErrorAction Continue
    }
}
