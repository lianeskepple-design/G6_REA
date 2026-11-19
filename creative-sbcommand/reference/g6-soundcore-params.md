# SoundCore 参数枚举（静态）

2026-11-19。来源：`C:\Program Files (x86)\Creative\Sound Blaster Command\Platform\Creative.Platform.Devices.dll` 的 `CTSoundCore._etParam*` 枚举。机器可读版本见 [g6-soundcore-params.json](g6-soundcore-params.json)。

## 用途与边界

这些是 **SoundCore API 的每特性参数偏移**，不是 APO 属性 PID，也不是
`5a 11` 通知里的设备侧 id。它们能说明“一个特性内部第几个参数是什么”，
但设备把哪些参数放进 `5a 11`、用什么全局 id，仍要由实机抓包确定。

## 关键布局

| 特性 | 参数 |
|---|---|
| `_etParamGraphicEQ` | 0 Enable，1 PreampGain，2–11 Band0..9 Gain，64 Bass，65 Treble |
| `_etParamCustomEQ` | 0 Enable，1 PreampGain，2–11 Band0..9 Gain，64/65 多段 Bass/Treble |
| `_etParamSmartVolume` | 0 Enable，1 Strength，2 Mode，3 PlusMode |
| `_etParamCrystalizer` | 0 Enable，1 Level |
| `_etParamCMSS3D` | 0 Enable，1 ImmersionLevel，2 ImmersionMode |
| `_etParamDialogPlus` | 0 Enable，1 Strength，2 Mode |
| `_etParamXBass` | 0 Enable，1 Strength，2 Freq_Hz |
| `_etParamSimpleSurround` | 0 Enable，1 Level |
| `_etParamEfxMasterControl` | 6 SCOUTModeMasterOnOff，7 SBXMasterOnOff |

GraphicEQ / CustomEQ 的 Band0..9 顺序对应
`{31,62,125,250,500,1000,2000,4000,8000,16000}` Hz（见
[g6-geq-pid-map.md](g6-geq-pid-map.md)）。

## 对旧抓包推断的修正

`capture/g6-usb-20261112/scout-01.pcap` 里 `id 05 = 0.74` 曾被解释为
“界面 SmartVol 74”。按本枚举：

- `_etParamSmartVolume` 只有 0–3，**没有 id 5**；
- 若通知是 GraphicEQ 特性内的偏移，id 5 是 **Band3 / 250 Hz 增益**；
- 若通知是全局设备表，则需要设备侧 id 表才能解释。

因此 “id 05 = SmartVol 74” 只能保留为**未验证的单次推断**，不能作为
USB id 映射的依据。`reference/g6-parameter-id-map.md` 已把该条标为中置信度，
本文件进一步说明它在 SoundCore 枚举里找不到对应项。

## 复现

```powershell
python -I tools/apo-eq-coeff/extract-soundcore-params.py `
  --json reference/g6-soundcore-params.json
```

脚本用 `dnfile` 只读解析元数据；缺少任一目标枚举会直接报错，不补零。

## 仍需实机

- `5a 11` 的全局 id 表（或“特性 + 偏移”的封装方式）。
- 十段逐段 +3 dB 时实际变化的 id。
- Scout GEQ 的十段增益值与硬件回环。
