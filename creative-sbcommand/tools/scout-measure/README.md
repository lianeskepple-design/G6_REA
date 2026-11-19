# Scout 取数工具（无设备可交付）

本目录提供两套 Scout 取数流程。所有脚本都可先用 `--self-test` 在无 G6 的机器上验证。
接回 G6 后的一键流程见 [hardware-runbook.md](hardware-runbook.md)，入口为
`run-g6-hardware.ps1`。

## 1. 扫频法测 Scout GEQ

### 生成扫频

```powershell
python -I generate-sweep.py --output-dir scout-measure-out
```

生成 48 kHz、20 Hz–20 kHz、8 s 的 Farina 扫频（float32 + PCM24）和 1 s 静音尾，
以及一个时间反转参考文件。报告写入 `sweep-report.json`。

### 录制 off/on

设备可用时：

```powershell
python -I measure-scout.py --record --device-match G6 --input-match "Line" --output-dir scout-measure-out
```

脚本会：

1. 找到名称含 `G6` 的播放设备和名称含 `Line` 的输入设备；
2. 播放扫频并录音，先提示在 Sound Blaster Command 里关闭 Scout，再提示开启；
3. 生成 `scout-off.wav`、`scout-on.wav`，然后自动分析。

WASAPI loopback 需要能录回放端点的输入设备；`sounddevice` 本身不提供 loopback，
可用物理回环线（G6 Line-out → 另一声卡 Line-in），或先用其他录音工具录好两个 WAV。

### 分析已有录音

```powershell
python -I measure-scout.py --analyze --sweep scout-measure-out/scout-sweep-float32.wav `
  --off off.wav --on on.wav --output scout-response.json
```

输出 10 个频段的 ΔdB、宽带 preamp 估计和 `net_db`。理论曲线为该软件模型的
`|H_scout_on| / |H_scout_off|`，不是实体 G6 的标定结果。

### 自测

```powershell
python -I measure-scout.py --self-test
python -I analyze-response.py --self-test
```

自测用与 C 交付相同的 APO 10 段 IIR 链合成已知曲线，验证反卷积能恢复
10 个频段（容差 0.6 dB）。

## 2. USBPcap 单频段采集

### 自测（使用已有抓包）

```powershell
python -I usbpcap-single-band.py --self-test
```

解析 `capture/g6-usb-20261112/scout-01.pcap` 中的 `5a 11` 通知，校验帧格式、
`len = 2 + 6×count`、8 条目帧和 `id 0 = 1.0`。

### 实机抓包

以管理员身份运行：

```powershell
python -I usbpcap-single-band.py --capture --interface "\\.\USBPcap1" --seconds 20 `
  --output scout-band.pcap
```

抓包开始后，在 Sound Blaster Command 里**只改一个 EQ 频段**（例如 31 Hz +2 dB），
等待抓包结束。脚本会解析所有 `5a 11` 帧并输出首末帧差异：

```json
{
  "changed": [{"id": 10, "before": 0.0, "after": 2.0}],
  "added": [],
  "removed": []
}
```

对 10 个频段各做一次，即可把 `96 <id>` 映射到频段；Scout 的 PID 5 是前级，
PID 10–19 是十段增益（来自 `CTTHXCommonPropKeys`）。

### 解析已有 pcap

```powershell
python -I usbpcap-single-band.py --parse --pcap capture/g6-usb-20261112/scout-01.pcap
```

## 依赖

- Python：`tools/audio-env/Scripts/python.exe`（numpy/scipy/soundfile/sounddevice）。
- Wireshark `tshark.exe`：默认 `C:/Program Files/Wireshark/tshark.exe`。
- USBPcap `USBPcapCMD.exe`：默认 `C:/Program Files/USBPcap/USBPcapCMD.exe`。
- 实机：G6 播放端点 + 可录回放的输入（物理回环或 loopback）。

## 边界

- 扫频结果包含 Scout 的 CMSS3D、GEQ、Reverb 整体差异；要分离 EQ，需在 UI 中
  单独开关其他效果，或按 `g6-scout-geq.md` 的窗口/扣除步骤处理。
- USBPcap 只抓主机到设备的参数通知，不证明设备固件实际执行了 EQ。
- 脚本不修改注册表、不写设备参数；`--record` 只播放/录音，`--capture` 只抓包。
