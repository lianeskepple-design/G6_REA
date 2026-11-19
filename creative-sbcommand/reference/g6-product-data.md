# Creative G6 产品数据（离线可得，不用插卡）

日期：2026-11-12 · 来源：本机 `C:\ProgramData\Creative\SBCommand\Product\G6\`
证据等级：**observed**（文件原样读取；未连接设备、未运行程序）

## 0. 离线复核后的结论

G6 与 AE-9 的预设字段和 EQ 频点结构相近，可以共享软件处理框架，但必须按设备与输出模式加载曲线。不能仅凭产品文件证明内部算法相同，也不能单凭 Reverb 关键词缺失判断 G6 无混响。

2026-11-12 复核：G6 的34组EQ中18组Speaker/Headphone不同；29组跨设备同名EQ，Speaker有6组、Headphone有7组不同。详见 factory-data-audit.json。

## 1. 产物

| 文件 | 内容 |
|---|---|
| `g6-profiles.md` | 23 个 profile 的 SBX 五档表 |
| `g6-eq-presets.md` | 34 个 EQ 预设的 10 段数值表 |
| `g6-eq-presets.json` | 同上，机器可读 |
| `g6-product-data.json` | profile + EQ 全量原始结构 |

## 2. Profile（23 个）

结构与 AE-9 完全同构（`Settings` 里 Speaker / Headphone 两条，字段仍是
Surround / Crystalizer / Bass / SVM / DialogPlus / Scout / SBXMaster），
多一个 profile 级字段 **`SpeakerMethod = 1`**（23个全是1）。其具体语义应以代码映射进一步确认。

G6 独有 `Gaming(10) / Music(20) / Movie(30)` 这类顺序档，其余游戏档 `Order = 100`。
`Bass.Level` 在 G6 里几乎一律 `0.50` 且 `Enable=false`（耳机不补低频），
这一点和 AE-9 相反——AE-9 是开 Bass 补。

### 同 id 预设里，G6 与 AE-9 差得最多的

| profile | 字段 | G6 | AE-9 |
|---|---|---|---|
| **FPS** | Surround / Crystalizer / Bass / SVM | **0.60 / 关 / 关 / 0.40** | 0.12 / 开 0.65 / 开 0.20 / 0.20 |
| Gaming | Surround / Bass | 0.12 / 关 | 0.67 / 开 0.10 |
| CS:GO | Surround / Crystalizer / SVM | 0.20 / **关** / 0.30 | **关** / 开 0.30 / 关 |
| PUBG | Surround | 0.45 | 0.15 |
| Overwatch | Surround | 0.67 | 关 |
| Dota2 | SVM / Dialog+ | 关 / 0.30 | 开 0.20 / 0.20 |

规律和 AE-9 内部那套"耳机 vs 扬声器"逻辑一致：**耳机向（G6）靠 Surround 立声场、
关掉 Crystalizer/Bass；扬声器向（AE-9）反过来靠 Bass 和 Crystalizer 补**。
FPS 这一档最典型：AE-9 给 0.12 的低环绕 + 0.65 的高 Crystalizer，
G6 给 0.60 的高环绕 + 全关 Crystalizer。

## 3. EQ 库：34组，每组包含Speaker与Headphone

原始JSON已逐项对照，设置完整且无差异。29组与AE-9同名，但Speaker有6组不同、Headphone有7组不同。旧结论“0/29有差异”已撤回。差异并集：ArenaOfValor、CounterStrikeGlobalOffensive、Dota2、FPS、Gaming、Overwatch、PlayerUnknown。

频点：31 / 62 / 125 / 250 / 500 / 1000 / 2000 / 4000 / 8000 / 16000 Hz。PreAmp均为0。Type为0或3。

### FootstepEnhancer

Speaker与Headphone均为：`[2, -4, -5, -3, -5, -3, 4, 5, -4, -4] dB`。频点与增益有原厂证据；软件EQ的Q和拓扑仍属于实现选择，不等同于Scout或硬件测量频响。

### 三组“全零预设”的更正

FPS、CallofDutySeries、DrivingSimulation只有Speaker全零，Headphone均非零：

| 预设 | Headphone十段增益，dB |
|---|---|
| FPS | [4, 3, 0, 0, 0, -2, -2, 0, 0, 2] |
| CallofDutySeries | [0, 3, -3, -3, -2, -2, 2, 0, 0, 0] |
| DrivingSimulation | [4, 4, 2, 2, -2, -2, 2, 0, 2, 0] |

完整双模式表见 g6-eq-presets.md；旧表只有Speaker，不能代表耳机曲线。

## 4. G6 专属程序集（`Creative.SBConnect2.G6.dll`，139 KB，托管）

39 个类型，页面 ViewModel：`G6SBXPage`、`G6AcousticEnginePage`、
`G6MicrophonePage`、`VoiceSettings`、`EqPage`、`DolbyDigitalPage`、`AudioPage`、
`SpeakerConfig`、`G6DeviceSettings`、`AvalonLighting`；另有 `AcquireAndUpdateFirmware`
（内置固件升级流程）。

Feature 清单（`Init*/set_*` 成套出现）：`SBXMaster`、`EqMaster`、`ScoutMaster`、
`Bass`、`StereoDirect`、`SPDIFDirect`、`SpeakerHRTF`、`HeadphoneLoad`、`headphoneGain`、
`outputMode`、`LED`、`UAC2`、`VoiceMorph`、`VCMaster`。

与 AE-9 程序集对比（字符串计数）：

| 关键词 | AE-9 | G6 | 结论 |
|---|---|---|---|
| `UAC2` | 0 | 2 | **G6 独有**：USB Audio Class 2 高采样率通路 |
| `VoiceMorph` | 1 | 2 | 两者都有 |
| `HRTF` | 4 | 6 | 两者都有 |
| `Dolby` | 12 | 9 | 两者都有 |
| `HeadphoneLoad` | 3 | 9 | 两者都有，G6 权重更高 |
| `Reverb` | 0（产品 DLL） | 0 | 仅说明产品DLL关键词未命中，设备实际能力未知 |

## 5. 对软件实现的约束

1. 可共享软件框架，但必须保留设备、预设、输出模式三个维度；硬件算法是否相同尚未证实。
2. SpeakerMethod=1是文件观察值；其准确含义应以代码映射为证，不能仅由名称下结论。
3. G6的FPS SBX profile与FPS EQ是两套设置，应分别加载。
4. FootstepEnhancer可提供软件EQ的频点与增益依据；Q和拓扑未知，不能声称硬件一致。
5. 设备是否支持混响、Scout的节点顺序与执行位置都仍需进一步证据。

## 6. 下一步（插上设备后）

- 枚举设备：VID/PID、USB 描述符、UAC2 是否启用、支持的采样率与位深
- 查 APO 链与属性存储：确认这些参数是走 APO 软件处理还是下发硬件
- 若要做协议：USBPcap 抓包 + 在 SB Command 里拖滑块，建立"参数→报文"映射
- 实测：`FootstepEnhancer` 曲线可以用扫频 + 回环验证是否与文件一致
