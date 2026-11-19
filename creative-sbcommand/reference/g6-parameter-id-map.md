# G6 USB 参数通知：`96 <id>` 编号映射

2026-11-12～13。全静态只读分析，未修改设备状态、未改 app/ 源码。机读版见[g6-parameter-id-map.json](g6-parameter-id-map.json)。

## 0. 结论

| 项 | 结果 | 置信度 |
|---|---|---|
| 报文结构 | `5a 11 <len> <count> 00 ( 96 <id> <float32 LE> )*`，且恒有 `len = 2 + 6×count` | 高（4 次独立命中） |
| id 空间 | 单一平坦 0..255，**没有 group/scope 命名空间** | 高 |
| `id 00` | SBX 主控开关（0.0 / 1.0） | 高 |
| `id 05` | SmartVol 归一化值（74 → 0.7400000095） | 中（单次观测） |
| 其余 id | **未找到**，不做猜测 | — |

## 1. 报文结构更正

旧笔记 `capture/g6-usb-20261112/scout-protocol.md` 把第 4 字节（从 1 计）当成 scope，实际是**条目数**：

| 来源文件 | 帧 | `len` | 第 4 字节 | 条目数 | `2+6×count` |
|---|---|---|---|---|---|
| sbx-master-02 | 37310 / 58555 | 0x08 | 0x01 | 1 | 8 ✓ |
| scout-01 | 22585 / 22587 | 0x32 | 0x08 | 8 | 50 ✓ |
| scout-01 | 22595 | 0x14 | 0x03 | 3 | 20 ✓ |
| scout-01 | 8217 | 0x0e | 0x02 | 2 | 14 ✓ |

四个样本单独看都可能被误读，四个一起看 `len = 2 + 6×count` 无一例外，故判定为条目数。

关于 `0x9600` 的两字节读法：与 `96|<id>` 算术等价，但 `0x96` 在所有观测中恒定，无法承载逐参数信息；全部原厂文件中也没有任何 `0x96xx` 字面量命名空间。因此按单字节 id 处理。

## 2. 已确证的映射

### id 00 = SBX Master OnOff（高）

- SBX 关闭帧（37310）：`id 00 = 0.0`
- SBX 开启帧（58555）：`id 00 = 1.0`
- 旁证：Scout 开启帧（8217）里 `id 00 = 0.0`，与 `SoundCoreScoutFeature` 反编译出的「Scout 会强制关闭 SBX 主控」完全一致。

这也**否定了**旧笔记里「id 00 是 Scout 启用位」的猜想——Scout 开启时它照样是 0。

### id 05 = SmartVol 归一化（中）

`0.7400000095` 在界面 SmartVol=74 时出现，与 74/100 吻合。但只有一次观测，未做「只改 SmartVol」的单变量差分，故只给中置信度。2026-11-19 复核 `_etParam*` 枚举：`_etParamSmartVolume` 只有 0–3，没有 id 5；若通知是 GraphicEQ 特性内偏移，id 5 是 Band3 / 250 Hz 增益。详见 [g6-soundcore-params.md](g6-soundcore-params.md)。因此该条仍只能算未验证推断，不能作为映射依据。

## 3. 观测到的 id 全集

`{00, 02, 04, 05, 07, 09, 0a, 0b, 0c, 0d, 0e, 0f, 10, 11, 12, 14, 18}`

除 00 与 05 外，所有值均为 0.0，无法据值推断语义。

## 4. 逐一对撞后全部排除的候选（负面证据）

| 候选命名空间 | 来源 | 关键显式值 | 结论 |
|---|---|---|---|
| `_etParamEfxMasterControl` | Interop.CtSndCr / Devices / Mixer | SCOUT=**6**，SBXMasterOnOff=**7** | 不符合 id 00/05 |
| `_etParamSmartVolume` | 同上 | Enable=0，Strength=**1**，Mode=2，Plus=3 | 不符合 id 05 |
| `etEfxMasterCtrlFeatBitPos` | Devices | SmartVolume=**2**（位序） | 不符合 |
| `MalcolmControlID` | Mixer | SBXMasterOnOff=**1314**，ScoutModeOnOff=1201 | 不符合，且属 AE-9 系 |
| `VT1728MALCOLMSCPQUERY_ParamType` / `etMiscellaneousSingleCommandParamType` | Devices / Mixer | 0..157，0=AECEnable，5=NoiseReductionStrength | **结构最接近**（设备级平坦参数表），但属 VT1728/AE-9，编号不符 |
| `APOINFO_HW_IDENTIFIER` / `ItemControlID` | 显式 | SmartVolume_Auto=9 等 | 不符合 |
| SndCrUSB.DLL 内的 `eParam*` 名 | MIDL 元数据（组块号+字符串偏移） | 只有名字，**无编号** | 仅证明参数存在 |

全库检索结论：没有任何枚举存在「值 0 且名字含 SBX」的成员，也没有「值 5 且名字含 SmartVol/Volume」的成员（唯一命中是无关的 `CTMFSESSIONPROCMSG_MEAudioSessionVolumeChanged=5`）。

## 5. 顺带排除的误读

写入命令 `5a 26 05 07 <01|02> 00 <val> 00` 中的 `0x07` 对 SBX 与 Scout **相同**，而 `_etParamEfxMasterControl` 里二者分别是 7 和 6。因此该字节是共享的命令选择子，**不是**参数编号。

## 6. 下一步

id 映射只靠静态文件确定不了，需要单变量差分：保持其他设置不变，只改一个参数（例如 SmartVol 74→50），抓包比对哪些 id 的值发生变化。这是唯一能给出确定映射的办法。

## 7. 方法与可复现

- 反编译 36 个托管程序集，用 `dnfile` 直接读 `Constant` 表，得到 1030 个带显式常量的枚举实际数值。注意 `ConstantRow.Value` 需按 `raw_data[1:]` 小端解码，否则数值全错；先用 `_etFeature`、`_etParamEfxMasterControl` 校准。
- 原生 `SndCrUSB.DLL`：解析 MIDL 类型库元数据块（`0x300000–0x390000`，441 条记录 / 79 个块）。记录结构 `[len][xx xx xx] <name> [u32 块号][u32 字符串偏移或 0xFFFFFFFF]`。
- 另扫描注册表、`C:\ProgramData\Creative`、`CTHID`/`CTHIDRpA`/`CTAudEp`/`CTCDC`/`CTMacroMgr`，以及固件文件（本机无 `.bin`/`.fw`）。
- 中间产物：`C:\Users\Administrator\AppData\Local\Temp\g6map\`（`enums_all.txt`、`meta_groups.txt`、`notif-summary.txt`）。
