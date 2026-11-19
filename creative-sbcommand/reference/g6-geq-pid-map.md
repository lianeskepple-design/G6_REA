# G6 十段增益：APO PID 顺序（静态）

2026-11-19。来源：`C:\Program Files (x86)\Creative\Sound Blaster Command\Platform\Creative.Platform.Devices.dll` 的 `Creative.Platform.Devices.Features.PropKeys.CTTHXCommonPropKeys` 静态构造函数。机器可读版本见 [g6-geq-pid-map.json](g6-geq-pid-map.json)。

## 结论

| 参数族 | APO PID | 频点（升序） |
|---|---|---|
| `CTPKEY_GraphicEQ_Gain0..9` | **0–9** | 31, 62, 125, 250, 500, 1000, 2000, 4000, 8000, 16000 Hz |
| `CTPKEY_Scout_GEQ_Gain0..9` | **10–19** | 31.25, 62.5, 125, 250, 500, 1000, 2000, 4000, 8000, 16000 Hz |

两套都是 `index 0 → 最低频`、`index 9 → 16 kHz`。这与
`GraphicsEqFeatureEnricher` 的 `numIndexes = CTPKEY_GraphicEQ_Gain9.pid + 1`
（= 10）以及 `GraphicsEqFeature` 的
`{31,62,125,250,500,1000,2000,4000,8000,16000}` 一致。

PID 按属性集 GUID 分组，数字只在各自 GUID 内有意义。Scout 全族共用
`CTPKEY_Scout_Guid`：

| PID | 字段 |
|---|---|
| 0 | `CTPKEY_Scout_Enable` |
| 1 | `CTPKEY_Scout_SVM_Enable` |
| 2 | `CTPKEY_Scout_SVM_Strength` |
| 3 | `CTPKEY_Scout_SVM_Mode` |
| 4 | `CTPKEY_Scout_GEQ_Enable` |
| 5 | `CTPKEY_Scout_GEQ_PreampGain` |
| 10–19 | `CTPKEY_Scout_GEQ_Gain0..9` |
| 20 | `CTPKEY_Scout_Reverb_Enable` |

## IL 证据

`CTTHXCommonPropKeys..cctor` 的 RVA 为 `0x181a8`（fat header，代码 7312 字节）。
每个频段是同一模式：

```text
ldsfld   CTPKEY_*_Gain_Guid      // 共用 GUID
ldc.i4   <index>                 // 频段索引
newobj   <indexed key ctor>
stsfld   CTPKEY_*_GainN
```

解出的存储点：

| 字段 | 构造索引 | 字段 | 构造索引 |
|---|---|---|---|
| `GraphicEQ_Gain0` | 0 | `Scout_GEQ_Gain0` | 10 |
| `GraphicEQ_Gain1` | 1 | `Scout_GEQ_Gain1` | 11 |
| … | … | … | … |
| `GraphicEQ_Gain9` | 9 | `Scout_GEQ_Gain9` | 19 |

复现：

```powershell
python -I tools/apo-eq-coeff/extract-geq-pids.py `
  --json reference/g6-geq-pid-map.json
```

脚本用 `dnfile` 只读解析元数据与 `.cctor` IL，不加载/执行目标程序集；
缺少任一字段或索引会直接报错，不会补零。

## 必须区分的两套编号

- **APO PID（本文件）**：主机侧 `CTTHXCommonPropKeys` 的属性编号，用于
  `IKsPropertySet`/APO 属性存储。
- **USB 参数 id**：`5a 11 <len> <count> 00 (96 <id> <float32>)*` 通知里的
  `<id>`，是设备侧编号。两者没有已验证的等价关系。

因此 `capture/g6-usb-20261112/scout-01.pcap` 里 11–20 全为 `0.0`
**不能**直接当作 Scout GEQ 的 `pid 11–19`；那批帧是 Scout 关闭后的
SBX 状态恢复，抓包期间没有逐段改动。USB id↔频段映射仍必须由
`tools/scout-measure/run-g6-hardware.ps1` 的逐段 +3 dB 抓包取得。

## 仍未取得

- G6 在线时 Scout GEQ 的实际十段增益值（需要实体设备，扫频或逐段读取）。
- USB `5a 11` id 与这 10 个频段的一一对应（需要逐段改 +3 dB 抓包）。
- Scout 的 CMSS3D / Reverb 节点参数与硬件回环。
