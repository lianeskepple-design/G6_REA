# G6 硬件取数 Runbook

本文件描述接回 G6 后的完整执行顺序。当前机器预检结果：
`Get-CimInstance Win32_PnPEntity` 没有 `VID_041E&PID_3256`，
`sounddevice` 也没有名称含 G6 的输出端点；脚本会以退出码 2 拒绝执行。

## 1. 前置条件

- G6 已连接并出现在 Windows 音频端点（Sound Blaster Command 能识别）。
- `tools/audio-env/Scripts/python.exe` 可用（numpy/scipy/soundfile/sounddevice）。
- Wireshark `tshark.exe`、USBPcap `USBPcapCMD.exe` 已安装，USBPcap 服务运行。
- 一个可录回放的输入：物理回环线（G6 Line-out → 另一声卡 Line-in）或可用的
  WASAPI loopback 输入。`sounddevice` 本身不做 loopback。
- 管理员权限（USBPcap 抓包）。

## 2. 预检

```powershell
pwsh -NoProfile -File tools/scout-measure/run-g6-hardware.ps1 -PreflightOnly -WaitSeconds 120
```

`-WaitSeconds` 会在 120 秒内每 5 秒轮询一次 PnP；插入/上电 G6 后自动继续。
预期输出 G6 名称/实例 ID 和 `PASS: preflight only`；没有设备时退出码 2。
本机当前注册表已有 `VID_041E&PID_3256` 的历史驱动键，但设备未枚举，
预检会明确打印这一点。

## 3. 一键跑完整套

```powershell
pwsh -NoProfile -File tools/scout-measure/run-g6-hardware.ps1 `
  -InputMatch "Line" `
  -OutputDirectory g6-hardware-out `
  -WaitSeconds 120
```

脚本依次执行：

1. **Scout 扫频**：提示在 SB Command 里关闭 Scout → 播放/录扫频；再提示开启 →
   第二次播放/录扫频。生成 `scout-off.wav`、`scout-on.wav`、`scout-response.json`。
2. **流程自检**：提示把图形 EQ 设为 Flat → 录一次；再只把 31 Hz 设为 +6 dB →
   录一次。要求 31 Hz 的 ΔdB 在 6 ± 0.8 dB 内，否则提示异常但不终止。
3. **USBPcap 单频段映射**：对 31/62/125/250/500/1k/2k/4k/8k/16k 每段各抓一次，
   每次提示“只改这一段到 +3 dB”，抓包结束后提示复位。解析首末帧差分，
   要求恰好一个 id 变化；结果写入 `g6-scout-id-map.json`。
4. **汇总**：`g6-hardware-report.json` 含预检、扫频结果、自检、id 映射。

单独跳过某步：`-SkipRecord`、`-SkipSanity`、`-SkipUsb`。

## 4. 手动分支

```powershell
# 只录 Scout off/on
python -I tools/scout-measure/measure-scout.py --record `
  --device-match G6 --input-match "Line" --output-dir g6-hardware-out --label scout

# 只分析已有录音
python -I tools/scout-measure/measure-scout.py --analyze `
  --sweep g6-hardware-out/scout-sweep-float32.wav `
  --off off.wav --on on.wav --output scout-response.json

# 只做单频段抓包
python -I tools/scout-measure/usbpcap-single-band.py --capture `
  --interface "\\.\USBPcap1" --seconds 15 `
  --output g6-hardware-out/band-31.pcap --report g6-hardware-out/band-31.json
```

## 5. 验收口径

- 扫频：同一状态重复两次，10 段差值 ≤ 0.3 dB；Flat 状态下 Δ = 0 ± 0.2 dB。
- 自检：31 Hz +6 dB 的测得值在 6 ± 0.8 dB。
- 映射：每个频段恰好一个 `96 <id>` 变化；10 段应得到 10 个不同 id（Scout PID 10–19），
  前级 PID 5 另记。
- 交叉：扫频得到的 10 段 ΔdB 与 USBPcap 映射出的 id 序列应一一对应；
  两者不一致时以扫频为准，并记录差异。

## 6. 当前阻塞

本机没有 G6（PnP 和音频端点均无 VID_041E/PID_3256），因此上述第 3 步之后的
实机数据尚未取得。脚本、解析器和自测均已交付；接回设备后按第 2–3 节执行即可。
