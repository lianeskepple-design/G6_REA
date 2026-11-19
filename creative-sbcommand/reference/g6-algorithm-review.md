# G6 音效调研 PDF 对照审阅

2026-11-19。审阅对象：`D:\工程\Sound BlasterX G6 声卡音效种类调研.pdf`（8 页）。
对照物：本项目对 `KSUSBAPO64.dll`、Sound Blaster Command 托管程序集、USB 抓包与现有 C/Rust 参考实现的证据。

## 一句话结论

PDF 的**产品层与算法分类**大体可信，和我们的 USB/APO 证据在“SBX/Scout 互斥、Scout 独立、Direct 旁路、5 个效果 + EQ”上对得上；但它把效果归到 **G6 板载 SB-Axx1 DSP**，而我们目前逐位移植的是 **Windows 侧 KSUSBAPO64 APO**。这两者是否是同一套算法，尚未被实机信号路径证明。PDF 更适合当“软件复刻路线图”，不能直接当 G6 硬件 DSP 的逐位验收标准。

## PDF 的核心结论

- G6 音效分两类：**SBX**（Surround、Crystalizer、Bass、Smart Volume、Dialog+）与 **Scout Mode**；二者在 DSP 层互斥，SBX 按键加载机身存储的整套预设，Scout 由独立按键控制；长按 Scout 进入 Direct 直通。
- 机身为 X-Amp 双功放、三级放大；音效由板载 **SB-Axx1** 硬件 DSP 处理，PC 端 Sound Blaster Command 只负责调参和把配置写入 G6。
- 22 个 SBX Profile；5 个核心滑块 + EQ。
- 算法分类：
  - Surround：HRTF 双耳滤波器组 + 多声道上混 + ITD/IID，专有 HRTF 数据集；仅耳机输出生效。
  - Crystalizer：非线性谐波再生，生成输入中不存在的高次分量，不是固定 EQ。
  - Bass：缺失基频（Missing Fundamental）+ 差频谐波合成，不是直接放大低频。
  - Smart Volume：多频段独立 AGC + 分频段 DRC + 峰值限幅，Normal/Loud/Night 三种模式。
  - Dialog+：2–4 kHz 带通增强 + 简单噪声抑制（维纳滤波类）。
  - Scout：STFT 短时傅里叶 → 瞬态事件检测 → 子带自适应增益 + 峰值保护；不是固定 EQ。
- 第 5–7 页用飞机、脚步、机枪、手榴弹、人声等素材给出 10–25 dB 的 Scout 动态差异。

## 对得上的部分

| PDF 说法 | 我们的证据 | 判断 |
|---|---|---|
| SBX 与 Scout 互斥，Scout 独立按键 | `ApoScoutFeature.Enable()` 会保存并关闭 SBX 主控、GraphicEQ、EAX、Reality3D；USB `5a 26 05 07 02` 与 SBX 的 scope 01 分开 | 一致 |
| SBX 按键加载机身预设 | USB 抓包里 Scout 开关与 SBX 总开关是两条同族命令；Scout 开启时 `id 00`（SBX 主控）变 0，关闭后恢复 1 | 一致（主机侧） |
| Direct 旁路全部音效 | 长按 Scout 进入 Direct 的产品行为；我们没有完整抓 Direct 状态，但协议上符合 | 基本一致，待抓包 |
| 5 个效果 + EQ | SoundCore 枚举有 GraphicEQ/CustomEQ、Crystalizer、CMSS3D、DialogPlus、SmartVolume、XBass；THX/Scout 节点另有 | 一致 |
| Scout 不是固定 EQ | 抓包和 PDF 都表明 Scout 会随内容变化；PDF 的扫频图也显示不同素材增益差 10–25 dB | 方向一致 |

## 关键分歧与未证实点

### 1. 硬件 DSP 还是主机 APO（最大风险）

PDF：所有音效在 G6 板载 SB-Axx1 上处理，PC 只调参。
我们：C 链来自 `KSUSBAPO64.dll`（Windows APO），包括 EQ、Crystalizer、CMSS3D/Surround、SVM 的代码级还原；USB 侧只验证了 Scout/SBX 开关和参数通知。

这意味着：
- 如果 G6 在硬件里跑另一套 DSP，我们的 C 链最多是“主机侧参考/近似”，不能声称与 G6 逐位一致。
- 如果 `KSUSBAPO64.dll` 就是 G6 在 Windows 下的处理路径（例如 Direct/某些模式），则 PDF 的“全部硬件 DSP”需要修正。
- 唯一能判定的是 G6 实机回环：分别测“USB 直通 + 设备 SBX 开/关”和“主机 APO 处理 + 设备 Direct”，比较两者脉冲/扫频。当前无 G6，此项阻塞。

### 2. Surround：HRTF FIR vs 我们的矩阵

PDF：HRTF 双耳滤波器组、ITD/IID、专有 HRTF 数据集，仅耳机输出。
我们：`g6_surround.c` 还原的是 CMSS3D 的**五通道增益矩阵**（0.9535/0.3015 = 17.5° 的 cos/sin、稳态/过渡斜坡），没有 HRTF FIR 表、没有 ITD/IID、没有耳机/线路输出路由判断。

判断：我们的 Surround 只是矩阵/声像层，**不是 PDF 描述的完整 HRTF 渲染器**。如果目标是软件复刻 G6 的空间感，需要补 HRTF 滤波器组或从 DLL/固件里找 HRTF 数据。

### 3. Crystalizer：非线性谐波 vs 我们的时变滤波

PDF：非线性谐波再生，生成输入不存在的高次分量。
我们：`g6_crystalizer.c` 是检测器驱动的时变一阶/二阶滤波 + 256 点 OLA/Hann/ramp；理论上时变滤波可以产生边带，但不是显式的谐波预测器。

实测（2 kHz 纯正弦，Crystalizer 100，48 kHz）：
- 旁路：谐波在 -160 dB 以下，broadband -171 dB。
- Crystalizer 100：1 kHz -39.6 dB、4 kHz -45.4 dB、5 kHz -39.6 dB、8 kHz -45.4 dB、10 kHz -50.8 dB；broadband -34.6 dB。
- 3 kHz/6 kHz 没有明显峰。

判断：我们的实现**确实生成新分量**（不是纯线性 EQ），但谱形不是干净的 2f/3f 谐波序列，更像时变滤波/调制产物。方向上与 PDF 的“非线性再生”不矛盾，但**具体算法未被证明是同一个**；需要与 G6 实机或 PDF 团队的测量对比。

### 4. Smart Volume：多频段 AGC+DRC+限幅 vs 我们的全带 AGC

PDF：多频段独立 AGC + 分频段 DRC + 峰值限幅；Normal/Loud/Night。
我们：`g6_svm.c` 是单检测器、全带增益律（`10^(-0.3·max(0, log10(det)-threshold))`），有攻击/释放/前瞻，但没有分频段、没有 per-band DRC、没有峰值限幅、没有模式曲线。

判断：我们的 SVM 是**全带简化版**，不是 PDF 描述的多频段系统。多频段行为可用双音/多音测试验证；当前实现不会出现“低频与高频独立压缩”。

### 5. Bass：缺失基频

PDF：Missing Fundamental + 2/3 次谐波合成。
我们：已加 `reference/g6_bass.c/h` 参考实现（`sbc-dsp --bass`、
`sbc-live` 的 Bass 控件），但**不是逆向还原**。APO 里已定位
`CBassBoost`/`CHspBassBoostState`/`EFXNODE_BASSBOOST` 等类/节点，
并进一步到 vtable/处理函数级：`CBassManagementEfxMod` vftable
`0x180188560`（+0x18 init、+0x40 process）、`CBassManager` process
`0x1800cc050`、10-tap FIR `0x1800cc560`、增益映射 `0x1800cb5e0`、
限幅 `0x1800cc310`、混合 `0x1800cb9e0`。

**修正**：`cc560`/`cb5e0` 作用在峰值增益包络上，不在音频上；
`CBassManagementEfxMod` 是限幅/动态链。真正的分频/路由核心在
`FUN_180089780`：每声道一阶**高通**（卫星声道去低频），同时把原始 6 声道
以 -15 dB 汇总到 LFE/sub——是 bass management，不是 Missing Fundamental
谐波合成，也不是低音提升。详见 [g6-bass-dialog-vtable.md](g6-bass-dialog-vtable.md)。
实测 60 Hz 输入、Bass 100：120 Hz -26.4 dB、180 Hz -39.9 dB。

判断：APO 的 Bass 分频/限幅两条链都已到函数级，但都不是 PDF 的
缺失基频算法；SBX Bass 更可能在 G6 硬件 DSP。

### 6. Dialog+：2–4 kHz + 维纳类抑制

PDF：带通增强 + 简单噪声抑制。
我们：已加 `reference/g6_dialog.c/h` 参考实现（`sbc-dsp --dialog`、
`sbc-live` 的 Dialog+ 控件），同样**不是逆向还原**。`KSUSBAPO64.dll`
没有 Dialog 字符串；`KSUSBSPI64.dll` 只有 `THXDialogueAdvantage_Amount/
isEnabled` 的参数处理器（`FUN_0044c1ac/FUN_0044c62c`），以及
`0x4078F0` 附近的处理器 vtable；这些只做属性读写，未找到 DSP 循环。
进一步解出 `CVocalSuppressorEfxMod`（vftable `0x18018de98`）是 4 段
可交叉淡化滤波器组，属性 GUID `{1ee40950-a46c-45eb-a50c-f1fd995c0166}`，
更像人声抑制/卡拉 OK，**不是** 2–4 kHz 对白增强，不能当作 Dialog+。
实测 1 kHz+3 kHz 等幅、Dialog 100：3 kHz 相对 1 kHz +5.8 dB。

判断：参考近似已可用；实际 DSP 仍未找到，可能在 G6 固件/硬件。

### 7. Scout：STFT 瞬态检测

PDF：STFT → 瞬态检测 → 子带自适应增益 + 限幅；不是固定 EQ。
我们：只做了 USB 开关/抓包和“Scout on/off 扫频”测量工具；没有 Scout 算法实现。当前 `run-g6-hardware.ps1` 用单次扫频对比，**不适合**测这种时变增益——扫频的每个频率只停留很短时间，Scout 的瞬态检测可能来不及响应，结果会把动态增益误当成频响。

判断：测量方法需要改成**重复瞬态脉冲串/分段素材**，做短时增益分析；算法实现是另一个大模块。

### 8. 22 个 SBX Profile vs 我们的 43 个 EQ 预设

PDF：22 个 SBX Profile（Apex、DOTA2、FPS、RPG 等）。
我们：`app/src/state.rs` 的 43 个预设是 EQ 预设/工厂曲线，不是同一层。SBX Profile 还包含 Surround/Crystalizer/Bass/SVM/Dialog+ 的整套参数。

判断：两层数据不要混用；需要把 22 个 SBX Profile 与 43 个 EQ 预设分开建模。

## PDF 的方法学局限

- 它是产品/算法层调研，不是代码或固件级证据；“HRTF FIR”“维纳滤波”“STFT”这些标签可能是根据行为/资料推断的，需要实测或反编译确认。
- 第 5–7 页的扫频图能证明“Scout 是内容自适应、不是固定 EQ”，但不能给出传递函数；Scout 的增益随时间变化，稳态扫频会失真。
- “全部在 SB-Axx1 硬件 DSP”与我们对 `KSUSBAPO64.dll` 的代码级证据存在冲突，必须用实机信号路径判定。

## 对项目的建议

**P0：先定信号路径。**
用 G6 回环分别测：
1. 设备 SBX 开/关 + USB 直通（设备 DSP 路径）。
2. 主机 APO 处理 + 设备 Direct（主机 APO 路径）。
比较脉冲/扫频/谐波。没有这一步，无法判断我们该复刻哪一套。

**P1：如果目标是软件客户端“像 G6”，按 PDF 补模块。**
优先级：
1. Bass（Missing Fundamental）——缺失且相对独立，容易先做。
2. Dialog+（2–4 kHz 带通 + 轻量噪声抑制）——缺失且独立。
3. Smart Volume 多频段 AGC/DRC/限幅 + 三模式——替换现有全带 SVM 或加模式层。
4. Surround HRTF FIR + ITD/IID——工作量最大，需要 HRTF 数据。
5. Scout STFT 瞬态检测——工作量最大，且必须实机对照。

**P2：改 Scout 测量。**
把单次扫频换成瞬态脉冲串/分段素材，记录短时增益；否则测不到 PDF 描述的 10–25 dB 动态差异。

**P3：客户端模式建模。**
` sbc-live` 目前允许 Crystalizer/SVM/EQ 任意组合，这不是 G6 的 SBX/Scout/Direct 模式语义。若要产品级一致，应增加：
- SBX 预设模式（整套参数，互斥 Scout）
- Scout 模式（独立，禁用 SBX）
- Direct（全旁路）
而不是把效果当独立开关。

## 结论

PDF 的**分类和方向**有价值，尤其是把 Bass、Dialog+、多频段 SVM、HRTF Surround、Scout STFT 点出来；但它不是我们 C 链的验收标准。当前项目真正完成的是：
- EQ 原厂系数（代码级、位级对照）
- Crystalizer 主机 APO 链（代码级、时变滤波；非线性方向一致，但未证明与硬件同算法）
- Surround 矩阵层（非 HRTF）
- Smart Volume 全带简化版
- Scout 只有 USB 开关/测量工具，无算法

下一步最值得做的是 P0 信号路径验证；硬件不在时，可以先补 Bass/Dialog+ 和 Scout 瞬态测量脚本。
