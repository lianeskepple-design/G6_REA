# Creative 平台参数范围（min / max / step / 默认值 / 缩放 / 调用链）— 离线静态分析

日期：2026-11-12（UTC+8）
范围：`C:\Program Files (x86)\Creative\Sound Blaster Command\`（Platform 下托管程序集 + 原生 DLL）、
`C:\ProgramData\Creative\SBCommand\Product\{AE9,G6}\`（出厂预设）。
方式：**只读**。未运行/未加载任何厂商 DLL，未改注册表，未连接设备。分析目标均为本地文件；ILSpy早期默认版本检查可能访问官方服务，后续已统一关闭更新检查。
证据等级：`static`（反编译/元数据/字节级读取）。任何"运行时才能确定"的值一律标 `unknown`。

配套文件：

| 文件 | 内容 |
|---|---|
| `reference/parameter-ranges.json` | 本报告的机器可读版本（含每条数值的出处） |
| `reference/offline-contracts/README.md` | 证据索引、复现命令、方法说明 |
| `reference/offline-contracts/sources.md` | 文件清单 + SHA256 + 程序集版本 + 工具版本 |
| `reference/offline-contracts/ranges-ui-constants.tsv` | 反编译源码中**全部** `EffectParameterRangeConfig` 调用点（file:line + 数值） |
| `reference/offline-contracts/call-chains.md` | 四条通路的逐级调用链（含 IL offset） |
| `reference/offline-contracts/snippets/*.cs` | 必要的定点反编译片段（10 个，均 ≤ 130 行） |

---

## 0. 结论摘要

1. **范围是上层常量，不是驱动查询。** 本次覆盖的、明确装配范围配置的可调效果参数的 `min/max/step/默认值`
   都硬编码在 `Creative.Platform.Devices.dll` 的 `*FeatureEnricher.EnrichUndetermined()` 中，
   以 `new EffectParameterRangeConfig<T>(min, max, granularity, default)` 的形式出现
   （`snippets/01`、`ranges-ui-constants.tsv`）。这是 **UI/框架常量层**。
2. **驱动侧确实有运行时范围查询接口，但 UI 不走它。** `Creative.Platform.Mixer.dll` 里存在
   `SoundCoreObject.GetParamInfoRange(MalcolmControlID, out def, out min, out max, out step, out attrib)`
   → `CtSndCr::GetParamInfo(_stParam, out _stParamInfo)`，`_stParamInfo` 结构里就是
   `minVal / maxVal / stepSize / defaultVal / paramAttrib / szDescription[32]`（`snippets/10`）。
   该 COM 服务的实现方是**驱动侧**（`SndCrUSB.DLL` 内部名 `KsMalcCtl.DLL`），
   所以这份范围**依赖具体硬件**，离线不可枚举 → 标 `unknown`。
   在 `Devices` 这条链路上（AE-9 / G6 的 SBX 面板）**没有任何调用点**读取它。
3. **`granularity` 不是硬约束。** `BasicNonIndexedEffectParameter.SetValue<T>()` 只做
   `min ≤ v ≤ max` 的边界检查（越界**静默丢弃**，不 clamp），**完全不读 granularity**
   （`snippets/02`，IL 见 `call-chains.md §5`）。granularity 只被 WPF ViewModel 用来生成
   slider 步长（`EqBandViewModel` / `AggregatedFeatureViewModel`）。所以"步进"是 UI 概念。
4. **缩放是"原值直通 + 显示归一"两层**：模型层用物理单位（dB / Hz / 0..1 / 毫贝），
   传输层 float 原样写入 COM / CTCDC / PROPVARIANT（无单位换算）；
   只有 WPF 层把 0..1 或物理值线性映射成 0..100 的百分比滑块
   （`AggregatedFeatureViewModel.CalculatePercentage`，`maxPercent = 100`）。
5. **当前定位到的两个设备相关分支**：`XBass.Strength` 的**量纲**（0..1 归一 vs 0..100 百分比，
   按仓储类型 + 产品代号分支），以及 Scout Mode 的**可用性**（按 APO EFX 节点存在与否）。

### 关键数值一览（均为静态常量，出处见 §3）

| 参数 | min | max | step | 默认 | 单位 | 作用域 |
|---|---|---|---|---|---|---|
| GraphicEQ Band0..9 Gain | −24 | 24 | 1 | 0 | dB | 全部设备 |
| GraphicEQ Preamp | −12 | 12 | 1 | 0 | dB | 全部设备 |
| GraphicEQ 频点 | 31/62/125/250/500/1k/2k/4k/8k/16k | — | — | — | Hz | 硬编码 10 段 |
| MicEQ Band0..9 Level | −24 | 24 | 1 | 0 | dB | X-Fi/VIP 设备 |
| MicEQ Preamp | −12 | 12 | 1 | 0 | dB | 同上 |
| MicEQ Band0..7 CenterFreq / Width | `unknown` | `unknown` | `unknown` | `unknown` | Hz / oct | 参数 id 存在，无范围常量 |
| XBass CrossOver | 10 | 1000 | 1 | 80 | Hz | X-Fi/VIP + APO |
| XBass Strength（SoundCore 硬件，非 Kratos2） | 0 | 1 | 0.01 | 0.5 | 归一 | 硬件 CtSndCr 通路 |
| XBass Strength（其它：CDC/APO、Kratos2） | 0 | 100 | 1 | 50 | 百分比 | CDC/APO 通路 |
| BassManagement CrossOver | 10 | 1000 | 1 | 80 | Hz | 除 G6 外的产品 |
| BassManagement SubwooferGain | 布尔（**不是**数值增益） | — | — | — | 开/关 | 除 G6 外 |
| EAX Environment Level | −6000 | 1200 | 10 | 0 | 1/100 dB（毫贝） | APO/硬件混响 |
| EAX3 Reverb 24 参数 | `unknown` | `unknown` | `unknown` | `unknown` | EAX4 惯例 | **无范围常量**（null config） |
| CMSS3D Surround（Immersion） | 0 | 1 | 0.01 | 0.4 | 归一 | — |
| Crystalizer Level | 0 | 1 | 0.01 | 0.65 | 归一 | — |
| Crystalizer PreAttenuation | 0 | 1 | 0.01 | 1 | 归一 | — |
| SmartVolume / MicSmartVolume Strength | 0 | 1 | 0.01 | 0.74 | 归一 | — |
| DialogPlus Level | 0 | 1 | 0.01 | 0.05 | 归一 | — |
| Scout Mode | 无数值参数 | — | — | — | — | 只有开关 + 热键 |

> ⚠️ **不要把这些数字当成"硬件允许范围"**。它们是**这一版 UI 允许你输入的区间**；
> 驱动实际接受的区间由 `GetParamInfo` 决定，静态不可知（§4）。

---

## 1. 证据基线与复现方式

完整清单（路径、大小、SHA256、程序集版本）见 `offline-contracts/sources.md`。要点：

| 程序集 | 版本 | SHA256（前 16） |
|---|---|---|
| `Platform\Creative.Platform.Devices.dll` | 2.4.67.0 | `9337d9cabc9c5de4…` |
| `Platform\Creative.Platform.Mixer.dll` | 2.4.67.0 | `e1f1cdcd2d79e8ba…` |
| `Platform\Interop.CtSndCr.dll` | 1.0.0.72 | `7956f40c9b67957a…` |
| `Platform\SndCrUSB.DLL`（原生，内部名 `KsMalcCtl.DLL`） | — | `564069833549063c…` |

工具（已装，未新增）：

- `ilspycmd 9.1.0.7988`（ICSharpCode.Decompiler 9.1.0.7988），经
  `C:\Program Files\dotnet\dotnet.exe --roll-forward Major tools\ilspycmd\tools\net8.0\any\ilspycmd.dll`
- `tools\audio-env\Scripts\python.exe`（3.12.10，dnfile 0.18.0 / pefile）

复现命令（Git Bash，工作目录 `D:\工程\逆向工程\creative-sbcommand`）：

```bash
DOTNET="C:/Program Files/dotnet/dotnet.exe"
ILSPY="D:/工程/逆向工程/creative-sbcommand/tools/ilspycmd/tools/net8.0/any/ilspycmd.dll"
PLAT="C:/Program Files (x86)/Creative/Sound Blaster Command/Platform"
OUT="$PWD/tools/.contract-repro"   # 项目内临时目录，复现后删除

# 1) 整程序集反编译（项目模式，一类型一文件）
"$DOTNET" --roll-forward Major "$ILSPY" --disable-updatecheck -p -o "$OUT/devices"        "$PLAT/Creative.Platform.Devices.dll"
"$DOTNET" --roll-forward Major "$ILSPY" --disable-updatecheck -p -o "$OUT/mixer"          "$PLAT/Creative.Platform.Mixer.dll"
"$DOTNET" --roll-forward Major "$ILSPY" --disable-updatecheck -p -o "$OUT/uifw"           "$PLAT/Creative.SBConnect.UI.Framework.dll"

# 2) 带 IL offset 的清单（-il -t 实际输出整个模块的 IL，用它取 offset）
"$DOTNET" --roll-forward Major "$ILSPY" --disable-updatecheck -il -t Creative.Platform.Devices.Features.Enrichers.XBassXFi23FeatureEnricher \
    "$PLAT/Creative.Platform.Devices.dll" > "$OUT/il_devices.txt"

# 3) 定位所有范围常量
grep -rn "new EffectParameterRangeConfig<" --include="*.cs" "$OUT/devices"
```

（Windows PowerShell 亦可：`& './tools/ilspycmd.cmd' -p -o <dir> <dll>`，已实测可用。）

---

## 2. 三层范围模型（这是本报告的核心结论）

```
                 ┌─────────────────────────────────────────────┐
 出厂预设 JSON   │ Product\AE9|G6\Presets|Profiles\*.json      │  观测值（observational）
 (观测，不是范围) │ 只是"原厂挑的点"，与允许范围无逻辑关系        │  → §5 / ranges.json.observed
                 └─────────────────────────────────────────────┘
                 ┌─────────────────────────────────────────────┐
 UI / 框架常量   │ Devices.*FeatureEnricher                    │  静态常量（static）★ 本报告主体
 (SB Command 用) │ EffectParameterRangeConfig<T>(min,max,step,default) │  → §3
                 └─────────────────────────────────────────────┘
                 ┌─────────────────────────────────────────────┐
 驱动运行时查询   │ Mixer.SoundCoreObject.GetParamInfoRange      │  硬件相关（runtime）
 (CtSndCr / 硬件)│ → CtSndCr::GetParamInfo → _stParamInfo        │  → §4，值 unknown
                 └─────────────────────────────────────────────┘
```

三条结论（都可用证据反驳或证实）：

- **A. 范围与预设无关。** 反例：`GraphicEQ` 允许 ±24 dB，而 35 个 AE-9 EQ 预设的
  观测极值只有 **−5.0 … +4.5 dB**（§5.1）。把 ±5 当成允许范围会直接砍掉 80% 的可调空间。
- **B. 范围与硬件无关（在 UI 侧）。** 同一份常量同时服务 AE-9 与 G6；设备差异只用
  "产品代号分支"改变**个别参数的量纲/存在性**（`XBass.Strength`、BassManagement、Scout 可用性），
  而不是改变区间数值。
- **C. 驱动侧范围不可静态获得。** `GetParamInfo` 是 COM vtable 调用，实现方是
  `KsMalcCtl.DLL`（= `SndCrUSB.DLL`）。离线只能说清**结构与调用位置**，说不出**数值**。

---

## 3. UI / 框架常量层（evidence: static）

结构定义（`snippets/01`）：

```csharp
public struct EffectParameterRangeConfig<T> {
    public T Min { get; } public T Max { get; } public T Granularity { get; } public T Default { get; }
    public EffectParameterRangeConfig(T min, T max, T granularity, T defaultVal)
}
```

装配点在 `Creative.Platform.Devices.Features.Enrichers.*FeatureEnricher.EnrichUndetermined()`；
Devices共56处范围构造位置的清单见 `offline-contracts/ranges-ui-constants.tsv`。

### 3.1 Graphic EQ（优先项 ①）

`GraphicsEqFeatureEnricher`（`snippets/03`，IL offset 见 `call-chains.md §5`）：

| 参数 ID | min | max | step | 默认 | 出处 |
|---|---|---|---|---|---|
| `GraphicEqBandLevelEffectParameterId`（indexed 0..9） | −24f | 24f | 1f | 0f | IL_004b/0050/0055/005a |
| `GraphicPreAmpEffectParameterId` | −12f | 12f | 1f | 0f | IL_00cc/00d1/00d6/00db |

- 段数 = `CTTHXCommonPropKeys.CTPKEY_GraphicEQ_Gain9.pid + 1` = **10**（`numIndexes`）。
- **频点同时出现在预设JSON和框架硬编码数组中**：
  `Creative.Platform.Devices.Features.GraphicsEqFeature.EqBandFrequecies = {31,62,125,250,500,1000,2000,4000,8000,16000}`
  （`snippets/05`）；`Creative.SBConnect.UI.Framework.ViewModels.Equalizer.EqBandViewModel.freqs` 同值。
  → 结论：**框架常量与预设数值相互印证**；此前从JSON提取频点本身有效，现在补充了独立代码证据。
- 边界：`-24/+24` 是 clamp 语义上的"UI 允许"，超界值在 `SetValue` 处被**丢弃**
  （不写、不 clamp、不报错），见 §5.3。

### 3.2 Mic EQ（优先项 ①'）

`MicEqFeatureEnricher`（`ranges-ui-constants.tsv` 同 file:10/14）：

| 参数 | min | max | step | 默认 | 备注 |
|---|---|---|---|---|---|
| `MicEqBandLevelEffectParameterId` | −24 | 24 | 1 | 0 | `numIndexes: 10`（IL_0078 `ldc.i4.s 10`） |
| `MicEqPreAmpEffectParameterId` | −12 | 12 | 1 | 0 | — |

**但驱动枚举里另有 8 段参量参数**：`_etParamVIP_MicEQ` 的
`Band0..7_CenterFreq = 10..17`、`Band0..7_Width = 18..25`。
在 `Devices` / `UI.Framework` / `Mixer` 三个程序集中**没有任何范围常量、也没有任何
写入调用点**引用它们 → `CenterFreq` / `Width` 的 min/max/step/default 一律 `unknown`
（唯一确定的是参数 id 与"每段独立可编程"这个事实）。

### 3.3 XBass（优先项 ②）

`XBassXFi23FeatureEnricher`（`snippets/04`）有两套装配逻辑，分支条件可以直接读出来：

```
if (CDC 仓储 != null && PropertyStore 仓储 != null)   → CDCApoXBassXFi23Feature（APO 路径）
      XOver = (10, 1000, 1, 80)
      Strength = (0, 100, 1, 50)            // 百分比
else                                                  → XBassXFi23Feature（SoundCore 硬件路径）
      XOver = (10, 1000, 1, 80)
      if (ProductCodeName != "G6")  加 BassMgmt XOver=(10,1000,1,80) + SubwooferGain(bool)
      Strength = 若 (仓储 is SoundCoreRepository && ProductCodeName != "Kratos2")
                     (0, 1, 0.01, 0.5)      // 归一
                 else (0, 100, 1, 50)       // 百分比
```

- **同一个旋钮两种量纲**，用 IL 常量可以逐位对上：`(0,1,0.01,0.5)` 与 `(0,100,1,50)`
  都在 `EnrichUndetermined` 里构造（IL_00ca..00d4 那组是 `(0,100,1,50)`）。
- `Kratos2` = `USB_BUS\FUNC_00&VEN_041E&DEV_0000&SUBSYS_30CA`（`Models.SupportedModels` 字典）。
  也就是说 **Kratos2走百分比；其它SoundCore仓储走归一；非SoundCore仓储走百分比**。
- `HasBassManagement(target) => target.DeviceEndpoint.ProductCodeName != "G6"`（同文件末尾）。
  G6 = `USB_BUS\FUNC_00&VEN_041E&DEV_0000&SUBSYS_3256`（`Models.SupportedModels`）。
  → **G6 没有 BassManagement 的 XOver / SubwooferGain 参数**，与出厂数据
  。预设中ArenaOfValor、BaseProfile、RPG的两套设置实际上Bass.Enable=true；不能把“缺少BassManagement”误读为“XBass始终关闭”。
- `XBassXFi23Feature`（`snippets/09`）里还有一条**参数重定向**：当扬声器配置 != HeadPhone
  且 `HasSubwoofer` 时，`XBassCrossOverFrequencyEffectParameterId` 被**替换**为
  `BassMgmtCrossOverFreqEffectParameterId`；开关 key 也从 `XBass` 换成 `BassManagement`。
  → 界面上同一个"分频"旋钮，落点随扬声器配置变化。复刻时若只做一条通路会不一致。

### 3.4 BassManagement（优先项 ③）

- `CrossOverFreq`：`(10, 1000, 1, 80)` Hz — 三处重复定义
  （`XBassXFi23FeatureEnricher` IL_007e/0083/0088/008d、`ApoBassRedirectionFeatureEnricher`、
  `XBassXFi23FeatureEnricher` 的 BassMgmt 分支）。**80 Hz 是默认值，不是硬编码上限。**
- `SubwooferBoost` 在本版是 **`ByValueEffectParameter<bool>`（布尔开关）**，
  参数 id `BassMgmtSubwooferGainId`（`snippets/04`）。**不是 0..N 的增益**——
  旧报告写"低音炮增益"容易被读成数值，实际是开关。
- 驱动枚举 `_etParamBassManagement` 还有 `SpeakerSize_FLFR/FCSW/RLRR/SLSR = 2..5`，
  但三个托管程序集里**没有对应的 PROPERTYKEY、没有 Feature 暴露、没有范围常量** → `unknown`。

### 3.5 Reverb（优先项 ④）

三条互不相通的东西，必须分开记：

**(a) `EAXEnvironment.Level`** — 唯一有范围常量的混响参数：

```
EaxEnvironmentFeatureEnricher:  new EffectParameterRangeConfig<long>(-6000, 1200, 10, 0)
ImmersionFeatureEnricher:       同上（第二份同样的常量）
```
IL：`IL_0058 ldc.i4 -6000`、`IL_005e ldc.i4 1200`、`IL_0064 ldc.i4.s 10`、`IL_0067 ldc.i4.0`
（`snippets/07`）。单位：EAX 惯例为 **1/100 dB（毫贝）**，即 −60.0 … +12.0 dB，步进 0.1 dB。
*（单位是按 EAX 惯例的推断，标注为 inferred；数值本身是 static。）*

**(b) `EAX3` 24 参数** — **完全没有范围**：

```
--types------Eax3FeatureEnricher: new BasicNonIndexedEffectParameter(..., pk, null)
                    ^^^^ 最后一个参数 = effectParameterRangeConfig = null
```
（`Devices.Features.Enrichers.Eax3FeatureEnricher.eax3EffectParams[24,3]`，
24 组 `Type / EffectParameterId / PROPERTYKEY`，`CreateNonIndexedEffectParam(..., null)`。）
后果：`BasicNonIndexedEffectParameter.SetValue` 在 `EffectParameterRangeConfig == null`
时**跳过边界检查**，任何值都会被写下去（`snippets/02` IL_002c `brfalse.s`）。
→ 这 24 个参数（Room / RoomHF / RoomLF / DecayTime / DecayHFRatio / DecayLFRatio /
Reflections / ReflectionsDelay / ReflectionsPan / Reverb / ReverbDelay / ReverbPan /
EchoTime / EchoDepth / ModulationTime / ModulationDepth / AirAbsorptionHF / HFReference /
LFReference / RoomRolloffFactor / Flags / Environment / EnvironmentSize / EnvironmentDiffusion）
**在 UI 侧无 min/max/step**；真实区间只能来自 §4 的驱动查询。**标 `unknown`。**

**(c) 麦克风混响（32 参数 / 19 预设，`eFeature_Effects_Reverb = 805306369`）** —
托管层**没有任何 Feature 装配它**（`Devices`/`Mixer`/`UI.Framework` 中 `eFeature_Effects_Reverb`
零引用；只有 `CTHIDRpALibrary.VT1728MALCOLMSCPQUERY_ParamType_MicReverb*` 的 X-Fi 旧枚举）。
它的 32 个参数只以**观测值**形式存在于 `SndCrUSB.DLL` 内嵌 XML（§5.3）→ 范围 `unknown`。

**(d) Immersion（CMSS 档位联动混响）** — 这是唯一能看到"参数如何被换算"的地方：

```
StartReverbThresholdPct = 0.5,  ReverbMaxPercentage = 0.8          (ImmersionFeature)
ShouldSetReverb:  surround > 0.5*(Max-Min)
GetReverbValue:   v = (surround - 0.5*(Max-Min)) / (Max - 0.5*(Max-Min))
                  reverb = v * ((Max_eax - Min_eax) * 0.8) + Min_eax
```
（`snippets/08`，数值全部 static。）代入 §3.5(a) 的区间：surround 从 0.5 → 1.0 时，
EAX Level 从 −6000 线性升到 `−6000 + 7200*0.8 = −240`（即 −60 dB → −2.4 dB）。
**这就是"沉浸度旋钮顺带加混响"的确切公式**，不是猜测。

### 3.6 Scout Mode（优先项 ⑤）

静态可得的部分（**全部是"开关/键位"，没有任何数值范围**）：

| 通路 | enable key | 证据 |
|---|---|---|
| SoundCore 硬件 | `_etFeature.eFeature_EfxMasterControl (1610612737)` + param index **6** = `eParamEfxMasterControl_SCOUTModeMasterOnOff` | `SoundCoreDeviceConfigCenter`（`"ScoutFeatureId" → (eFeature_EfxMasterControl, 6u)`）；原生侧同名描述串 `eParamEfxMasterControl_SCOUTModeMasterOnOffW0` / `Scout Mode Master Control`（`SndCrUSB.DLL`） |
| APO | `PROPERTYKEY {F0C45844-CE1D-4D97-B5EC-AA7A22A71613} pid 0` (`CTPKEY_Scout_Enable`) | `ApoDeviceRepoKeyFactory`、`CTTHXCommonPropKeys` |
| CDC-only 设备 | 仅一个 `BasicToggleableFeature`（无子链） | `ScoutModeFeatureEnricher` 第二分支 |

- **子链**：`SoundCoreScoutFeature` 持有 Surround / Crystalizer / XBass / SVM / DialogPlus /
  GraphicsEQ 六个 feature 的引用；`Enable()` 记录各自状态后打开 SCOUT 主开关，
  `Disable()` 只关主开关（独立完整片段见call-chain-crosscheck/SoundCoreScoutFeature.cs.txt）。
  → 此托管封装可证行为是**主开关与六个子开关的状态快照**；不能由此排除底层另有专属DSP。
- **APO 侧可用性由 EFX 节点门控**（真·硬件相关）：
  `ApoFeatureChecker["ScoutFeatureId"] = IsSupported(EFXNODE_THX_SCOUT_CMSS3D | EFXNODE_THX_REALITY3D_SCOUT_CMSS3D)
   && IsSupported(EFXNODE_THX_SCOUT_GRAPHICEQ | EFXNODE_THX_REALITY3D_SCOUT_GRAPHICEQ)`。
  `ApoScoutFeature` 另有 `MasterOnOff = 主开关`、`EAXEnvironment`（混响）联动。
- **Scout 的 GEQ 数值仍然 `unknown`，但这次能说清它为什么 unknown**：
  `CTTHXCommonPropKeys` 定义了 `CTPKEY_Scout_Guid` 的
  `Scout_SVM_Enable(pid1) / _Strength(2) / _Mode(3) / _GEQ_Enable(4) / _GEQ_PreampGain(5) /
  _GEQ_Gain0..9(pid10..19)`，**但三个托管程序集里没有任何读取/写入这些 key 的代码**；
  `UI.Framework.CMDViewModels.ScoutMode.BaseScoutModePageViewModel` 只做"开关 + 全局热键"
  （`scoutFeatureViewModel = new AggregatedFeatureViewModel(aggregatedFeature)`），
  页面上没有 EQ 曲线控件。`CTAudEp.dll` 内 `Scout` 字符串命中数 = 0。
  → **所查托管程序集未找到曲线值读写，CTAudEp.dll字符串未命中也不能排除其内部处理；底层宿主与曲线仍未知。**
  频点同样 `unknown`。

### 3.7 其余（同一次扫描所得，供交叉核对）

| 参数 | min | max | step | 默认 | 单位 |
|---|---|---|---|---|---|
| Crystalizer Level | 0 | 1 | 0.01 | 0.65 | 归一 |
| Crystalizer PreAttenuation | 0 | 1 | 0.01 | 1 | 归一 |
| DialogPlus Level | 0 | 1 | 0.01 | 0.05 | 归一 |
| SmartVolume / MicSmartVolume | 0 | 1 | 0.01 | 0.74 | 归一 |
| CMSS Surround（Immersion） | 0 | 1 | 0.01 | 0.4 | 归一 |
| NoiseReduction | 0 | 1 | 0.01 | 0.5 | 归一 |
| MicBoost | 0 | 30 | 10 | 0 | dB（步进 10 dB） |
| Calibration Level | −20 | 20 | 1 | 0 | dB |
| Calibration Delay | 0 | 0.015 | 0.001 | 0 | s |
| VoiceFocus WedgeAngle | 20 | 180 | 1 | 30 | 度 |
| MultiplexOutput | 1 | 4 | 1 | 2(Rendering)/1(CDC) | 枚举 |
| SpeakerConfigFirmware | 1 | 8 | 1 | 2 | 枚举 |
| HeadphoneGain / UAC2 / SPDIF 等布尔项 | 0 | 1 | 1 | 0 | bool |

---

## 4. 驱动运行时 `GetParamInfo` 层（evidence: static 结构 + unknown 数值）

这条链路是**硬件相关范围的唯一来源**，必须说清"它存在、怎么调"，但不能给数。

```
Creative.Platform.Mixer.MalcolmControl.GetParamInfoRange(MalcolmControlID, out def, out min, out max, out step, out attrib)
   └─ SoundCoreObject.GetParamInfoRange(...)                       [IL 见 call-chains.md §5]
        ├─ NewParamID(MalcolmControlID) → CTSoundCore._stParam{paramId, featureId, contextId}
        ├─ GetParamInfo(_stParam) → 缓存 Hashtable（LoadFeatureInfo 时用 EnumParams 预填）
        │     └─ 未命中/需刷新 → scCtrl.GetParamInfo(param, out _stParamInfo)
        └─ 按 _stParamInfo.paramType 取 floatVal / dwordVal / longVal
             eParamType_Float = 0 → floatVal 分支（IL_005a..0084 连续读取 defaultVal/minVal/maxVal/stepSize）
```

- `_stParamInfo`（`snippets/10`）= `{paramId, paramType, paramDataSize, minVal, maxVal, stepSize,
  defaultVal, paramAttrib, szDescription[32]}` —— **驱动自己就返回 min/max/step/default**，
  语义与 UI 的 `EffectParameterRangeConfig` 完全对应。
- `CtSndCr` 接口 `CTSoundCore.ISoundCore`（`Interop.CtSndCr.dll`，IID
  `6111E7C4-3EA4-47ED-B074-C638875282C4`）的 vtable 顺序为：
  `BindHardware, EnumContexts, GetContextInfo, GetContext, SetContext, EnumFeatures,
  [_VtblGap1_1], EnumParams, GetParamInfo, GetParamValue, SetParamValue,
  [_VtblGap2_4], GetParamValueEx_unsafe, SetParamValueEx_unsafe, …`。
- **实现方是原生 COM 服务**：`SndCrUSB.DLL` 的导出只有
  `DllCanUnloadNow / DllGetClassObject / DllInstall / DllRegisterServer / DllUnregisterServer`，
  文件内部 DLL 名为 `KsMalcCtl.DLL`（即 Kernel-Streaming "Malcolm" 控制）。参数范围随硬件/固件变。
- **两个调用侧都不用它来定 UI 区间**：
  - `Devices` 的 `ISoundCore` 副本里**连 `GetParamInfo` 都没有**（只有
    `GetParamValue / SetParamValue / GetParamValueEx_unsafe / SetParamValueEx_unsafe` +
    `_VtblGap1_2 / _VtblGap2_3 / _VtblGap3_4` 空洞）；
  - 全仓 grep `GetParamInfoRange|GetParamInfo(` 的调用点只有 `MalcolmControl` 自身
    （X-Fi 时代的 `Malcolm` 混音器面板），SB Command 的 SBX 面板不走这里。
- 因此：**想要真实硬件区间，必须在线（或离线加载 COM）调用 `GetParamInfo`；
  本次按约束不做，标 `unknown`。** 复现方法记在 `offline-contracts/call-chains.md §6`（含参数 id 组装规则）。

---

## 5. 原厂预设观测层（observational，不是范围）

一切"预设极值"都只能当**采样点**，不能当区间。

### 5.1 EQ 工厂预设（AE-9 35 个 / G6 34 个）

`C:\ProgramData\Creative\SBCommand\Product\{AE9,G6}\Presets\EQ\*.json`，`Unit: "dB"`，
`Bands[10].Frequency` 固定 31…16000，**与 §3.1 的硬编码数组一致**（互为交叉验证）。

| 设备 | 频段值观测区间 | 非整数点数 | PreAmp |
|---|---|---|---|
| AE-9（35 文件） | −5.0 … +4.5 dB | 83（如 ±2.5 / ±4.5 / ±3.5） | 全部 0.0 |
| G6（34 文件） | −5.0 … +5.0 dB | 56 | 全部 0.0 |

→ **预设的分辨率是 0.5 dB，UI 的 granularity 是 1 dB。** 两者不是一回事，
旧报告"步进 0.1 dB"在任何一层都找不到依据（详见 §7 勘误）。

### 5.2 SBX Profile（AE-9 25 / G6 23）

`Profiles\*.json` 的 `Settings[Type=Speaker|Headphone]`。数值字段只在
`Surround.Level / Crystalizer.Level / Bass.Level / SVM.Level` 上：

| 设备 | Level 观测区间 | XOver |
|---|---|---|
| AE-9 | 0.00 … 0.74（250 个采样） | 全部 80 |
| G6 | 0.00 … 0.74（230 个采样） | 全部 80 |

→ 全部落在 §3.7 的 `0..1` 归一区间内；`XOver` 恒等于 §3.4 的默认值 80 Hz。
**"25 个 profile 全是 80 Hz"只能说明原厂没改过默认，不能说明 80 是唯一可选。**

### 5.3 麦克风混响 19 预设 × 32 参数（字节级复核）

`SndCrUSB.DLL`，`<SoundCorePresets>` 位于 offset `0x9738`，长度 **49,455 B**，
`FeatureId` 全为 `805306369`（= `eFeature_Effects_Reverb`）。本次独立重算
（19 个预设名与旧报告完全一致，每组32条：ParamId 1–31加`ActivePreset(258)`，共437Float、152Bool、19Integer；不含Enable）：

| ParamId | 名称 | 19 预设观测 min | max | 不同取值数 |
|---|---|---|---|---|
| 1 | RoomLevel | −10.0 | 0.0 | 2 |
| 2 | RoomHF | −60.0 | 0.0 | 12 |
| 3 | RoomLF | 0.0 | 0.0 | 1 |
| 4 | DecayTime | 0.17 | 14.67 | 18 |
| 5 | DecayHFRatio | 0.10 | 1.54 | 14 |
| 6 | DecayLFRatio | 0.96 | 1.788 | 7 |
| 7 | ReflectionsLevel | −45.0 | 10.0 | 18 |
| 8 | ReflectionsDelay | 1.0 | 100.0 | 10 |
| 9 | ReverbLevel | −100.0 | 17.0 | 18 |
| 10 | ReverbDelay | 2.0 | 93.0 | 11 |
| 11 | Size | 1.0 | 21.6 | 10 |
| 12 | Diffusion | 37.0 | 100.0 | 5 |
| 13 | AirAbsorptionHF | −0.07 | −0.05 | 2 |
| 14/16/17/18/19 | *Scale 系列 | 1.0 | 1.0 | 1 |
| 15 | DecayHFLimit | 0.0 | 1.0 | 2 |
| 20 | HFReference(=HFCutOff) | 5000.0 | 5030.0 | 2 |
| 21 | LFReference(=LFCutOff) | 250.0 | 253.0 | 2 |
| 22 | ModulationRate(=DetuneRate) | 0.25 | 17.5 | 6 |
| 23 | ModulationDepth(=DetuneDepth) | 0.0 | 100.0 | 3 |
| 24 | EchoDepth | 0.0 | 100.0 | 4 |
| 25 | EchoRate | 4.0 | 13.0 | 5 |
| 26/27 | *RateScale | 0.0 | 1.0 | 2 |
| 28 | ReflectionsPanAzimuth | 0.0 | 127.0 | 2 |
| 29 | ReflectionsPanDiverge | 25.0 | 100.0 | 2 |
| 30 | ReverbPanAzimuth | 0.0 | 0.0 | 1 |
| 31 | ReverbPanDiverge | 100.0 | 100.0 | 1 |

**量纲必须分开说（旧报告有一处明显误读）**：

- `DecayTime` 观测值 0.17…14.67 —— 这在**秒**的量级；若按"毫秒表示"读会得到 0.17 ms，
  与 `Recording Studio`（0.17 s）"最短"的语义矛盾。→ **推断为秒（inferred）**。
- `ReflectionsDelay / ReverbDelay` 观测 1…100 / 2…93 —— 毫秒量级 → **推断为 ms（inferred）**。
- `HFReference = 5000…5030`、`LFReference = 250…253` —— 是 **Hz**，不是 0..1 归一量。
- `Size = 1.0…21.6` —— 既不是 EAX 惯例的 0..100，也不是 0..1；**单位/量纲 unknown**。
- `Diffusion / EchoDepth / ModulationDepth = 0…100` —— 百分比（inferred）。

**这 32 个参数的 min/max/step 全部 `unknown`**（驱动不导出，托管层未装配）。

---

## 6. 参数缩放（real value ↔ percent ↔ transport）

### 6.1 模型层：物理单位，无换算

`SoundCoreRepository.SetValue<T>`（`Devices.Features.SoundCore`）按 T 直接填
`_stParamValue`：

| T | paramType | 写入 |
|---|---|---|
| `float` | `eParamType_Float (0)` | `paramVal.floatVal = (float)val` |
| `int/uint` | `eParamType_Dword (2)` | `paramVal.dwordVal` |
| `long/ulong` | `eParamType_Dword (2)` | `dwordVal = (uint)val`（**注意：long 被截成 32 位**） |
| `bool` | `eParamType_Bool (1)` | `boolVal = 0/1` |

**没有任何 dB↔归一、Hz↔归一、百分比↔归一的换算**。UI 给什么，硬件就收什么。
（这解释了为什么 `XBass.Strength` 必须靠"换区间"而不是"换算"来切换量纲。）

### 6.2 UI 层：物理值 → 0..100 百分比滑块

`Creative.SBConnect.UI.Framework.ViewModels.AggregatedFeatureViewModel`
（字段 `maxPercent = 100`、`minPercent = 0`）：

```csharp
public virtual int CalculatePercentage(double value)
    => (int)((double)((float)(Maximum - Minimum) / (realMaximum - realMinimum)) * (value - realMinimum) + 0.5);
private float CalculateValue(int percent)
    => (realMaximum - realMinimum) / (float)(Maximum - Minimum) * (float)percent + realMinimum;
```

- 百分比**只存在于 WPF 绑定层**（`Value` 是 int 0..100）；写回时用 `CalculateValue`
  还原成物理值再交给模型层。
- `Step = CalculatePercentage(rangeConfig.Granularity)`：所以 granularity 的可见效果是
  "滑块上一步走多少格"。对 0..1 归一参数（step 0.01）→ 1 格 = 1%；对 EQ（step 1 dB，区间 48 dB）
  → 1 格 ≈ 2%。
- **例外（EQ 面板不走百分比）**：`ViewModels.Equalizer.EqBandViewModel` 直接把
  `Value = GetValue<float>()`、`Step = rangeConfig.Granularity` 用物理 dB 显示，
  所以 AE-9 的 EQ 滑块步进就是 **1 dB**。

### 6.3 传输层：float 原样

| 通路 | 载体 | 类型标记 |
|---|---|---|
| SoundCore（CtSndCr COM） | `_stParamValue{paramType, paramVal}` | `eParamType_Float` |
| CDC（CTCDC.dll） | `CTCDCCMDPARAM_SetMalcolmEffectValue.flValue` | 参数类型 `CTCDCMALCOLMEFFECT_ParamType_*`，indexed 时 `paramType += i` |
| APO（Windows property store） | `PROPVARIANT` | `vt = 4 (VT_R4)` for float、`11` bool、`3` i4、`19` ui4、`22` i4、`23` ui4 |

（`Devices.Features.CDCDevices.CDCRepositoryKeyFactory` / `Devices.Features.Apo.PropStoreRepository`
的 `PropStoreSetDict`。）

---

## 7. 调用链（摘要；完整版见 `offline-contracts/call-chains.md`）

### 7.1 通路 A：SoundCore 硬件（AE 系列、G6 走 USB）

```
WPF ViewModel（EqBandViewModel / AggregatedFeatureViewModel）
  → AggregatedFeature / Feature（Devices.Features.GraphicsEqFeature / XBassXFi23Feature…）
  → BasicNonIndexedEffectParameter.SetValue<T>()        ← 边界检查只查 min/max
  → IDeviceRepository.SetValue → SoundCoreRepository.SetValue<T>
  → CTSoundCore.ISoundCore.SetParamValue(_stParam, _stParamValue)   （COM）
  → KsMalcCtl COM 服务（SndCrUSB.DLL）→ 驱动 → 硬件
```

### 7.2 通路 B：CDC（CTCDC / Malcolm 效果值）

```
… → CDCDeviceRepository.SetValue<T> → CDCRepositoryKey<G,S>.SetValue
  → CTCDCLibrary.CTCDCCMD.SetMalcolmEffectValue(CTCDCCMDPARAM_SetMalcolmEffectValue)
      paramType = CTCDCMALCOLMEFFECT_ParamType_GraphicEQBand0Gain (+ index)
      srcID     = CTCDCMALCOLMEFFECT_SourceID_NotApplicable
      flValue   = v        ← 原值（dB）
```

### 7.3 通路 C：APO（Windows 属性存储，`PropstoreEp = 0 / PropstoreFx = 1`）

```
… → PropStoreRepository.SetValue → PROPVARIANT(vt=4, AsFloat=v)
  → Creative.Platform.PropStore.PolicyConfigPropertyStore.SetPropertyValue(endpointId, type, PROPERTYKEY, pv)
  → 音频引擎属性存储 → APO（EFX 节点）
```

### 7.4 通路 D：运行时范围查询（未被 SBX 面板使用）

```
MalcolmControl.GetParamInfoRange(MalcolmControlID)
  → SoundCoreObject.GetParamInfoRange → NewParamID → GetParamInfo(_stParam) → _stParamInfo
  → scCtrl.GetParamInfo(param, out paramInfo)  ← CTSoundCore.ISoundCore，实现方 KsMalcCtl
```

---

## 8. 设备差异（静态可证的四条）

| 差异 | 判据 | 影响 |
|---|---|---|
| 无 BassManagement | `ProductCodeName != "G6"` | G6 不装配 BassMgmt XOver / SubwooferGain |
| `XBass.Strength` 量纲 | `repo is SoundCoreRepository && ProductCodeName != "Kratos2"` | 归一 vs 百分比 |
| Scout 可用性 | APO EFX 节点 `EFXNODE_THX_SCOUT_{CMSS3D,GRAPHICEQ}`（含 Reality3D 变体） | 无节点 → 无 Scout 页 |
| 分频旋钮落点 | `SpeakerMode != HeadPhone && HasSubwoofer` | XBass.XOver ↔ BassMgmt.XOver 重定向 |

**补充核实**：G6产品程序集确实存在：`C:/ProgramData/Creative/SBCommand/Product/G6/Creative.SBConnect2.G6.dll`（139760字节）。此前只搜索Program Files得出“未安装”的结论错误，现已撤回。来源SHA256见JSON与offline-contracts/sources.md。

---

## 9. 勘误（对 `reference/creative-dsp-parameters.md` / `g6-product-data.md`）

| 旧说法 | 本次结论 | 依据 |
|---|---|---|
| "EQ 单位 dB，**步进 0.1**" | 模型 granularity = **1 dB**；预设文件分辨率 **0.5 dB**；`0.1` 无出处 | `GraphicsEqFeatureEnricher` IL_0055 `ldc.r4 1`；EQ JSON 出现 ±2.5/±4.5/±3.5 |
| "10 段频点来自 AE9 工厂预设 JSON" | 频点是**框架硬编码常量**（Devices `GraphicsEqFeature` + UI `EqBandViewModel` 两处），JSON 只是同值 | §3.1 |
| "`BassManagement` … `SubwooferBoost`（增益）" | 本版是 **bool 开关** `ByValueEffectParameter<bool>` | `snippets/04` |
| "混响 … 时间 s 的**毫秒表示**" | `DecayTime` 观测 0.17…14.67 → **秒**；`*Delay` 才是 ms | §5.3 |
| "参数范围（min/max/step）仍缺：需要展开 Devices 的托管代码" | 已展开：**范围不在 Devices 缺，而是齐备**（§3）；真正缺的是驱动侧 §4 与 MicEQ 频宽/EAX3 | 本报告 |
| "Scout 数值零命中（HKLM/APOINI/user.config）" | 结论仍成立，但**原因升级**：托管层只做"开关+热键"，C# 里连读写 Scout GEQ key 的代码都没有；`CTAudEp.dll` 也零命中 | §3.6 |
| "60 组参数枚举" | 本机 **51–52 组** `_etParam*`（Devices 51 / Interop 52 / Mixer 51） | `ls CTSDK/ \| grep ^_etParam` |
| "G6 与 AE-9 共用同一 EQ 库（29/29 值不差）" | 已由factory-data-audit.json逐文件复核：29个同名预设中Speaker有6个、Headphone有7个不同，旧结论已撤回 | §3.1 |

---

## 10. known / unknown 台账

**KNOWN（static）**

- 本次已找到的UI/框架侧min/max/step/default（§0 表 + `ranges-ui-constants.tsv`）。
- EQ 10 段频点；EQ 段数 10；MicEQ 段数 10。
- XBass / BassManagement 的分支条件与两套量纲常量。
- `granularity` 不参与校验（只用于 UI 步长）。
- `EAXEnvironment.Level` 区间与 Immersion 映射公式；CMSS 阈值 0.5 / 上限 0.8。
- Scout 的三条 enable key（硬件 feature+param6 / APO PROPERTYKEY / CDC 单开关）与 APO EFX 门控。
- 三条传输通路的类型编码（float/dword/bool；VT_R4 等）。
- 19 个混响预设的 32 参数观测值（含量纲推断）。

**UNKNOWN（需要在线或原生逆向）**

- 驱动 `_stParamInfo` 的真实 min/max/step/default（逐参数）。
- MicEQ 8 段 `CenterFreq / Width` 的范围（参数 id 10..25）。
- EAX3 24 参数的范围（托管层 config = null）。
- 32 个麦克风混响参数的 min/max/step；`Size` 的量纲。
- Scout GEQ十段增益与频点：所查静态证据不足以恢复。
- BassManagement `SpeakerSize_*`（驱动 enum 索引 2..5）的取值范围/含义。
- G6硬件运行时能力及实际DSP效果，产品程序集存在不能替代设备实测。

---

## 11. 下一步边界（不越界）

1. **不加载、不调用任何厂商 DLL**——所以 §4 的真实区间只能靠"在线对照"取得：
   插上 AE-9/G6，用 DebugView 抓 `Util.PrintDebug("Get Param info …")`
   （`SoundCoreObject.GetParamInfoRange` 里就有这条日志，格式
   `Get Param info <ID>: Default x / Min x / Max x / Step x / Attribute x`）。
   ← 这是最小成本路径：**原厂自己把范围打进日志了**。
2. Scout 曲线：下一步应定位 APO/EFX 节点实现文件（不是 `Platform\CTAudEp.dll`）；
   在 `EFXNODE_THX_SCOUT_GRAPHICEQ` 的宿主 DLL 里找 GEQ 系数。当前无证据 → 保持 unknown。
3. 原生侧参数描述表：`SndCrUSB.DLL` 里能看到形如
   `@THXXBass_Amount`、`THXBassMgt_CrossOverFreq`、`Scout Mode Master Control` 的
   UTF-16 串与"名称 + 定长字段"记录簇（offset ≈ `0x1EABC` 附近），
   疑似持久化/属性表描述符。**未解析**（需要 disasm 确认字段语义），不作为结论。
4. 本报告不涉及 `app/` 的任何改动，也不修改既有报告。

---

## 12. 复现清单

```bash
# 范围常量（本报告 §3 的每一行）
grep -rn "new EffectParameterRangeConfig<" --include="*.cs" "$OUT/devices"

# IL 常量（§3.1 / §3.3 / §3.5 的 offset）
"$DOTNET" --roll-forward Major "$ILSPY" --disable-updatecheck -il -t Creative.Platform.Devices.Features.Enrichers.GraphicsEqFeatureEnricher "$PLAT/Creative.Platform.Devices.dll"
"$DOTNET" --roll-forward Major "$ILSPY" --disable-updatecheck -il -t Creative.Platform.Devices.Features.Enrichers.XBassXFi23FeatureEnricher   "$PLAT/Creative.Platform.Devices.dll"
"$DOTNET" --roll-forward Major "$ILSPY" --disable-updatecheck -il -t Creative.Platform.Mixer.SoundCoreObject                                "$PLAT/Creative.Platform.Mixer.dll"

# 运行时范围接口（§4）
grep -rn "GetParamInfoRange\|_stParamInfo" --include="*.cs" "$OUT/mixer"

# 混响预设（§5.3）：字节级定位 + 独立解析
#   offset 0x9738，<SoundCorePresets> … </SoundCorePresets>，49,455 B
#   解析脚本见 offline-contracts/README.md §"预设复核脚本"

# 出厂 EQ / Profile 观测区间（§5.1 / §5.2）
#   见 offline-contracts/README.md §"预设复核脚本"
```
