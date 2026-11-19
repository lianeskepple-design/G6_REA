# Creative Sound Core DSP：滤波器与混响参数（实物提取）

日期：2026-11-12 · 目标机器：本机安装的 Sound Blaster Command（AE-9）
证据等级：**observed**（全部来自已安装文件，未运行原程序、未抓包）

---

## 1. 结论先说

1. **参数在托管互操作层**：`Platform/Interop.CtSndCr.dll` 里有 60组参数枚举，
   描述本程序集声明的相关参数id；不代表设备全部私有参数均已覆盖。
   这是 1:1 的 API 契约，不是猜测。
2. **预设值在原生驱动层**：`Platform/SndCrUSB.DLL`（3.99 MB，原生 PE）内嵌一段
   `<SoundCorePresets>` XML，共 **19 个混响预设 × 32 条记录 = 608 条值记录**（437 Float、152 Bool、19 Integer；每组为参数1–31加258 ActivePreset，不含Enable），
   带 FeatureId/ParamId。已整段裁出，见 `soundcore-presets.xml`。
3. **算法位置尚未证实**：已提取的枚举与预设XML提供参数接口和值，尚未还原滤波器Q、系数、混响拓扑或硬件输出。关键词未命中不能证明DLL不含实现，也不足以判定软件/硬件执行位置。软件实现必须明确与原厂算法的证据边界。

---

## 2. 证据来源与提取方法

| 对象 | 大小 | 性质 | 提取物 |
|---|---|---|---|
| `Platform/Interop.CtSndCr.dll` | 34,816 B | 托管（P/Invoke） | 60 个参数枚举的常量值 |
| `Platform/SndCrUSB.DLL` | 3,988,480 B | 原生 x64 | 内嵌预设 XML（offset `0x9738`，49,455 B） |
| `Platform/Creative.Platform.Devices.dll` | 647,680 B | 托管 | 设备侧封装（未展开） |
| `C:\ProgramData\Creative\SBCommand\Product\AE9\Presets\EQ\*.json` | 35 个 | JSON | 均衡器工厂预设（Speaker / Headphone 各一套） |

方法：
- 托管程序集：`dnfile` 读 .NET 元数据，直接取 `Constant` 表的枚举字面量
  （不是从字符串猜顺序，是取真值）。
- 原生 DLL：字节级定位 `<SoundCorePresets>` 到 `</SoundCorePresets>`，
  整段（49,455 B）原样导出，XML 结构完好可直接解析。
- 生成物：`creative-param-ids.txt`（全部 60 组枚举）、
  `mic-reverb-presets.json`（机器可读）、`mic-reverb-presets.md`（参数×预设矩阵）。

---

## 3. 参数契约总览（Interop.CtSndCr.dll，60 组枚举）

与"滤波器/混响"直接相关的部分：

| 枚举 | id 范围 | 用途 |
|---|---|---|
| `_etParamReverb` | 0–31 有值，256–260 预设操作 | 主混响（EAX4 风格 32 参数） |
| `_etParamGraphicEQ` | 0–11、64、65、256–260 | 10 段图形 EQ + 前级 + Bass/Treble |
| `_etParamCustomEQ` | 同上 | 自定义 EQ（另一套 10 段） |
| `_etParamVIP_MicEQ` | 0–17+ | 麦克风 8 段 EQ，每段独立中心频率/宽度 |
| `_etParamXBass` | 0–2 | XBass：开关 / 强度 / 频率 Hz |
| `_etParamBassManagement` | 0–6 | 分频点 Hz、各声道喇叭尺寸、低音炮增益 |
| `_etParamSpeakerEnhancement` | 0–4 | 扬声器 EQ 使能 / 预设 |
| `_etParamCrystalizer` | 0–1 | 谐波增强：开关 / 级别 |
| `_etParamDialogPlus` | 0–2 | 对白增强：开关 / 强度 / 模式 |
| `_etParamSmartVolume` | 0–3 | 智能音量：开关 / 强度 / 模式 / Plus 模式 |
| `_etParamCMSS3D` | 0–2 | CMSS-3D：开关 / 沉浸度 / 模式 |
| `_etParamSimpleSurround` | 0–1 | 简化环绕：开关 / 级别 |
| `_etParamSurroundType`、`_etParamEAX3` | — | 环绕类型与 EAX3 环境 |
| `_etParamPitchShift`、`_etParamVIP_VoiceFX` | — | 变调 / 麦克风人声效果 |
| `_etParamVIP_NoiseReduction`、`_etParamLineNoiseReduction` | — | 麦克风 / 线路降噪 |
| `_etParamEfxMasterControl`、`_etParamProcessingControl` | — | 效果总开关 / 直通 |

完整 60 组（含蓝牙、灯光、电池等非音频项）见 `creative-param-ids.txt`。

### 3.1 混响 32 个参数（`_etParamReverb`，值得全抄）

```
0  Enable                1  RoomLevel            2  RoomHF
3  RoomLF                4  DecayTime            5  DecayHFRatio
6  DecayLFRatio          7  ReflectionsLevel     8  ReflectionsDelay
9  ReverbLevel          10  ReverbDelay         11  Size
12 Diffusion            13  AirAbsorptionHF     14  DecayTimeScale
15 DecayHFLimit         16  ReflectionsLevelScale  17 ReflectionsDelayScale
18 ReverbLevelScale     19  ReverbDelayScale     20  HFReference (=HFCutOff)
21 LFReference (=LFCutOff)  22 ModulationRate (=DetuneRate)
23 ModulationDepth (=DetuneDepth)  24 EchoDepth  25 EchoRate
26 ModulationRateScale (=DetuneRateScale)  27 EchoRateScale
28 ReflectionsPanAzimuth  29 ReflectionsPanDiverge
30 ReverbPanAzimuth       31 ReverbPanDiverge
256 EnumPreset  257 EnumAllPresets  258 ActivePreset  259 SavePreset  260 RemovePreset
```

21/22/23 是**别名**（`HFReference` 与 `HFCutOff` 同值 20），这点在枚举里写死了，
说明同一参数在两代 API 里叫法不同——复刻时按一套名字实现即可。

---

## 4. 混响预设（19 个，值全部来自 DLL 内嵌 XML）

FeatureId 全部为 `805306369`（0x30000001，麦克风混响功能）。
预设名与关键参数（单位按 EAX 惯例：电平 dB、时间 s 的毫秒表示、Delay 为 ms）：

| 预设 | DecayTime | RoomHF | ReflectionsLevel | ReflectionsDelay | ReverbLevel | ReverbDelay | Size | Diffusion |
|---|---|---|---|---|---|---|---|---|
| Auditorium | 4.32 | −4.76 | −7.89 | 20 | −2.89 | 30 | 21.6 | 100 |
| Ball Park | 14.67 | 0.00 | 7.8 | 88 | −100.0 | 76 | 7.5 | 100 |
| Bath Room | 1.49 | −12.0 | −3.7 | 7 | 10.3 | 11 | 1.4 | 100 |
| Cathedral | 3.72 | −5.1 | −16.1 | 20 | −4.8 | 40 | 7.5 | 100 |
| Chapel | 2.01 | −28.3 | −45.0 | 5 | −3.6 | 12 | 10.0 | 75 |
| Church | 2.21 | −5.1 | −11.1 | 20 | −6.7 | 40 | 7.5 | 100 |
| Concert Hall | 3.92 | −5.0 | −12.3 | 20 | −0.02 | 29 | 19.6 | 100 |
| Default | 3.00 | −1.0 | 10.0 | 100 | 2.0 | 20 | 5.0 | 100 |
| Garage | 1.41 | −4.0 | 3.3 | 26 | −9.1 | 12 | 10.0 | 80 |
| Haunted Cavern | 5.43 | −5.1 | −3.9 | 20 | −4.2 | 40 | 7.5 | 50 |
| Indoor Arena | 2.61 | −5.1 | −13.3 | 20 | −7.9 | 40 | 2.0 | 37 |
| Jazz Club | 1.21 | −5.1 | −1.7 | 6 | 7.3 | 93 | 10.0 | 100 |
| Living Room | 0.50 | −60.0 | −13.76 | 3 | −11.04 | 4 | 2.5 | 100 |
| Opera House | 2.91 | −5.1 | −7.8 | 20 | −7.9 | 40 | 7.5 | 100 |
| Psychotic | 7.56 | −1.51 | −6.26 | 20 | 13.74 | 30 | 1.0 | 50 |
| Recording Studio | 0.17 | −60.0 | −12.04 | 1 | 2.07 | 2 | 1.4 | 100 |
| Small Room | 0.40 | −4.5 | −16.1 | 2 | 4.2 | 3 | 2.0 | 100 |
| Theater | 1.11 | −5.1 | −12.8 | 20 | −9.7 | 40 | 7.5 | 100 |
| Underwater | 1.49 | −40.0 | −4.49 | 7 | 17.0 | 11 | 1.8 | 100 |

每组参数1–31与ActivePreset=258，共32条 × 19预设的完整矩阵：`mic-reverb-presets.md`；机器可读：`mic-reverb-presets.json`。

几个可复用的观察：
- `RoomLevel` 除 `Default` 外全是 0，说明原厂只靠 `RoomHF/RoomLF` 塑形，不整体衰减。
- `Ball Park` 的 `ReverbLevel = −100 dB` 是**故意关掉混响尾巴**，只留反射（室外场）——
  这是一个"同一条链上只留早期反射"的实例，复刻时不能用统一混响模型糊过去。
- 最短 `Recording Studio`（0.17 s / Size 1.4）对最长 `Ball Park`（14.67 s）差 **86 倍**，
  动态范围要求很高。

---

## 5. 滤波器侧参数

### 5.1 图形 / 自定义均衡器（`GraphicEQ` / `CustomEQ`）
```
0 Enable   1 PreampGain   2..11 Band0..Band9 Gain   64 BassLevel   65 TrebleLevel
256..260 EnumPreset / EnumAllPresets / ActivePreset / SavePreset / RemovePreset
```
频段（来自 AE9 工厂预设 JSON，Speaker 与 Headphone 两套一致）：
`31 / 62 / 125 / 250 / 500 / 1k / 2k / 4k / 8k / 16k Hz`，单位 dB；UI使用0.1步进，设备允许步进应另按参数查询调用链确认。
工厂 EQ 预设共 35 个（`Presets/EQ/*.json`），每个含 Speaker + Headphone 两组
`PreAmp` 与 10 段 `Value`。示例 Cinema：`[0,0,0,0,2,3,0,0,1,1]`, preamp 0。

### 5.2 麦克风 EQ（`VIP_MicEQ`，8 段，带可编程频点）
`Enable / PreampGain / Band0..7_Level / Band0..7_CenterFreq / Band0..7_Width`
——注意它和图形 EQ 不一样：**中心频率与带宽都是参数**，可以做成真正的参量均衡。

### 5.3 低频
- `XBass`：`Enable / Strength / Freq_Hz`（3 个参数，最省事的一条低音增强链）。
- `BassManagement`：`CrossOverFreq_Hz`（默认 80 Hz，范围 10–1000）、
  `SpeakerSize_FLFR / FCSW / RLRR / SLSR`、`SubwooferBoost`。

### 5.4 其余"效果"参数（都是 1–4 个旋钮的薄封装）
`Crystalizer`（级别）、`DialogPlus`（强度+模式）、`SmartVolume`（强度+模式+Plus 模式）、
`CMSS3D`（沉浸度+模式）、`SimpleSurround`（级别）。
**这类参数只有"给多少"，没有"怎么算"** —— 具体算法在驱动里，DLL 只透传。

---

## 6. 扬声器预设（`Profiles\*.json`，25 个 profile，已全部读完）

这是 SndCrUSB 里那份麦克风混响之外的**另一半**：SBX 的扬声器/耳机配置。
`C:\ProgramData\Creative\SBCommand\Product\AE9\Profiles\` 共 25 个 JSON，
每个文件里 `Settings` 有两条：`Type 0` = 扬声器、`Type 1` = 耳机。字段固定七个：

```
Surround{Enable,Level}      Crystalizer{Enable,Level}     Bass{Enable,Level,XOver,SubWooferGain}
SVM{Enable,Level,Mode}      DialogPlus{Enable,Level}      Scout{Enable}      SBXMaster{Enable}
```

**关键发现：这25个profile里没有Reverb字段。** 这只能说明该预设集合没有保存此字段，不能据此证明扬声器通路不存在混响。已提取的19组XML明确位于MicReverb节点。

`XOver` 25 个 profile 全部是 80 Hz（与 `BassManagement_CrossOverFreq_Hz` 默认值一致）。

耳机与扬声器不是复制粘贴，有 9 个 profile 两套设置不同，规律很明显：

| profile | 字段 | 扬声器 | 耳机 |
|---|---|---|---|
| Battlefield1 | Surround / Bass 开关 | 0.20 / 开 | **0.67** / **关** |
| CallOfDuty | Surround / Bass / SVM 开关 | 0.12 / 开 / 开 | **0.67** / 关 / **关** |
| Racing | Bass 开关 | 关 | **开** |
| Dota2 | Surround 开关 / Dialog+ | 关 / 开 | **开** / 关 |
| Overwatch | Surround 开关 | 关 | **开** |

规律：**耳机下 Surround 一律拉高（多数 0.67）、低频相关（Bass）多半关掉**；
扬声器反过来靠 Bass 补低频。复刻版的"扬声器/耳机两套预设"不该做成同一份数据。

工厂预设的八个主档与调查结论 fnd_2 完全一致（Gaming .67/.30/.10/.74、
Cinema .67/.55/.20/.74、Concert .67/.50/.40/.60、Night .67/.55/.20/.74 + SVM 模式 2、
FPS .12/.65/.20/.20、RPG .35/.50/.20/.30、Racing .30/.50/.50/.40、
Strategy(RealTimeStrategy) .67/.30/.50/.50），等于给 fnd_2 补上了原始文件级证据。
另外还有 17 个游戏专属 profile（Apex、CS:GO、Dota2、Fortnite、LoL、PUBG、Overwatch、
Rocket League、The Witcher 3、MGSV、Project Cars、Stadium…），它们的 `Order` 都是 100。

`SpeakerEqConfigs\*.cfg` 共 33 个，但内容只有两行：`model <型号名>` 和 `order 50`——
**没有频响曲线、没有系数**。所谓"扬声器 EQ 校准"在原厂这里是机型标签，
真正的补偿曲线不随包分发（在设备/驱动里）。

完整两套表格：`speaker-profiles.md`；机器可读：`speaker-profiles.json`。

### 6.1 Scout Mode（"射击/听脚步"模式）的真实结构

> 注意：UI 里没有叫"射击模式"的项。容易混的是两个东西——
> **`第一人称射击`（FPS 预设）** 和 **`Scout Mode`**（中文界面保留英文，
> 描述是"增强游戏中的脚步声线索"）。两者实现方式完全不同：前者只是参数集，
> 后者是一条独立的效果链。

**FPS 预设**（`Profiles/FPS.json`，扬声器与耳机两套**完全相同**，25 个里少数如此）：
`Surround 0.12 / Crystalizer 0.65 / Bass 0.20 / XOver 80 / SVM 0.20（Mode 0） /
DialogPlus Enable=false（但存档值 0.50，即"配好未启用"）/ Scout off / SBXMaster on`。
它没有任何专属算法——就是把 SBX 五个档位调到特定数值。

**Scout Mode** 在 `Creative.Platform.Devices.dll` 里露出了完整骨架（43 处命中）：

| 类别 | 内容 |
|---|---|
| 设备参数键 | `CTPKEY_Scout_Enable`、`_GEQ_Enable`、`_GEQ_PreampGain`、`_GEQ_Gain0..9`、`_Reverb_Enable`、`_SVM_Enable`、`_SVM_Mode`、`_SVM_Strength`、`_Guid`、各 `_Last_State` |
| 实现类 | `IScoutFeature` → `SoundCoreScoutFeature`（走 CtSndCr 硬件）、`ApoScoutFeature`（走 Windows APO 音频处理对象）、`AggregatedScoutFeature`（两者聚合） |
| EFX 节点 | `EFXNODE_THX_SCOUT_GRAPHICEQ`、`EFXNODE_THX_SCOUT_REVERB`、`EFXNODE_THX_SCOUT_CMSS3D`（各有 Reality3D 变体） |
| 顶层开关 | `eParamEfxMasterControl_SCOUTModeMasterOnOff = 6` |
| 热键 | `DefaultHotKeyState = false`——原厂不给默认手势，由用户自己指（中文资源："您要将此热键重新指定为 Scout Mode 吗"） |

上述键名与节点表明Scout接口涉及GEQ、预增益、混响、CMSS3D和SVM。它们的执行顺序、当前设备实际启用子集以及软件/硬件分工仍需代码或设备证据，不能只从节点命名断言严格串联。

**拿不到的部分**：`CTPKEY_Scout_GEQ_Gain0..9` 的**数值**不在任何可读位置——
`HKLM\SOFTWARE\Creative`、`ProgramData\Creative\APOINI\*.ini`、
`user.config` 全部查过，零命中；10 个频段的**中心频率也没有定义**。
这与混响不同：混响的 19×32 有实数，Scout 的曲线只能自己标定。

另外一个容易忽略的事实：**25 个出厂 profile 里 `Scout.Enable` 全是 false**，
没有一个是"预设就打开"的——它是纯手动开关。

---

## 7. 当前尚未还原的实现

| 项目 | 已知边界 |
|---|---|
| EQ系数、Q、拓扑 | 原厂频点与增益已提取；系数与带宽未证实 |
| 混响结构与IR | 已有预设值，尚未找到或验证具体拓扑/IR |
| 实际采样率与位深 | UI和产品能力声明不能代替设备支持格式实测 |
| DSP执行位置 | 软件接口、APO、原生组件和硬件之间的具体分工未确认 |

此前关键词普查的未命中仅作为搜索记录，不能排除优化、无符号或不同命名的实现。撤回“算法不存在”“已排除其他DLL”“波形不可能一致”等过度肯定说法。目前能承诺的是参数数据有出处，软件算法必须独立说明并验证。

## 8. 在复刻版（`app/`）里的落地建议

按证据强度排优先级：

1. **10 段图形 EQ 先上**（证据最硬）：频点 31–16k、dB 值、preamp 全是实数；
   实现 = 10 个 RBJ peaking biquad 串联 + 末尾前级增益。工厂 35 个预设可直接读 JSON 灌入。
2. **麦克风 EQ 8 段**：有 CenterFreq/Width，可以直接做成参量 EQ（比图形 EQ 更"真"）。
3. **XBass 3 参数**：低频搁架 + 强度插值，最简单的一环，能把"低音增强"从假旋钮变成可听。
4. **BassManagement 分频**：80 Hz 默认、10–1000 可调；LR4 分频 + 低音炮增益。
5. **混响按 19 预设参数表实现**：先用固定拓扑（4 comb + 2 allpass 或 FDN），
   把 `RoomLevel/RoomHF/DecayTime/DecayHFRatio/Reflections*/Reverb*/Size/Diffusion`
   映射到结构参数；`Ball Park` 那种 ReverbLevel=−100 的极端预设必须能表达"只留反射"。
6. **Crystalizer / DialogPlus / SmartVolume / CMSS3D / SimpleSurround**：只有强度参数，
   没有算法。要么标注"估计实现"（谐波激励 / 中频提升 / 响度压缩 / 立体声展宽），
   要么保持 UI-only 并明示。**不要在文档里写成"已按原厂实现"。**

---

## 9. 未做 / 下一步

- `Creative.Platform.Devices.dll`、`Creative.Platform.Mixer.dll` 的托管代码未展开
  （旧调查阶段仅元数据；现已安装ILSpy，补充结果见parameter-ranges.md），里面可能有参数范围（min/max/step）与默认值。
- `SndCrUSB.DLL` 的原生函数未反编译（4 MB，需 Ghidra）。若要找"驱动侧到底怎么算"，
  从这里入手：搜引用 `Reverb_DecayTime` 等字符串的函数，看是否有本地 DSP 实现。
- 产品预设 `Presets/EQ/*.json`（35 个）已确认结构与频点，但尚未全部导入复刻版
  （复刻版目前只有自己的 15 个内部预设）。
- `Profiles/*`（25 个）与 `SpeakerEqConfigs/*`（33 个）**已读完**，见 §6。
- 参数范围（min/max/step）仍缺：需要展开 `Creative.Platform.Devices.dll` 的托管代码，
  后续结果见parameter-ranges.md；需区分静态UI范围与设备运行时范围。
