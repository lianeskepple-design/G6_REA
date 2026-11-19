# Bass 与 Dialog+：逆向状态与参考实现

2026-11-19。回答“有没有逆向出实际算法”这个问题：**Bass 的 APO 限幅链已到
函数级，但 Bass 提升本身未定位；Dialog+ 只到参数处理器级**。下面记录代码级
证据，以及为了先能听/能测而做的参考实现。

## 结论

| 效果 | 逆向状态 | 参考实现 |
|---|---|---|
| Bass | APO 已定位两条链：分频/路由核心 `FUN_180089780`（卫星声道高通 + 原始低频汇总到 sub）与限幅链（包络→dB→软拐点→平滑→增益）；不是 Missing Fundamental | 有（Missing Fundamental） |
| Dialog+ | 只定位到 THX 参数处理器 vtable；`CVocalSuppressorEfxMod` 是 4 段滤波器组候选，未证实 | 有（2–4 kHz + 轻量门） |

参考实现仍是**按调研 PDF 的算法类别写的近似**，不是 `KSUSBAPO64.dll`/G6 固件的逐位还原。
vtable 地址、槽位、常量和处理链见 [g6-bass-dialog-vtable.md](g6-bass-dialog-vtable.md)。

重要修正：`FUN_1800cc560` 的 10-tap FIR 和 `FUN_1800cb5e0` 的三次多项式
作用在**增益包络**上，不是音频谐波整形。`CBassManagementEfxMod` 是限幅器，
不能当作 Missing Fundamental 的逐位实现。

## Bass 的代码级证据

`KSUSBAPO64.dll` 里有 RTTI/节点名：

- `.?AVCBassBoost@@`
- `.?AVCBassBoostMixerBase@@`
- `.?AVCBassBoostMixerLFE@@`
- `.?AVCBassBoostMixerSmallSpeakers@@`
- `.?AVCHspBassBoostState@@`
- `EFXNODE_BASSBOOST`（节点 7）、`EFXNODE_BASSMGT`（节点 6）、
  `EFXNODE_THX_BASSMGMT`（节点 21）、
  `EFXNODE_THX_REALITY3D_BASSMGMT`（节点 44）

这说明 Bass 在 Windows APO 里有完整的类/节点结构。后续 vtable 级逆向解出的
`CBassManagementEfxMod` 处理链包含两段：
1. 分频/路由核心 `FUN_180089780`：每声道一阶**高通**（状态为上一输入/上一输出，
   `H(z)=c(z⁻¹-1)/(1+a2z⁻¹)`，截止 `cutoff/sample_rate`），把卫星声道低频去掉；
   同时把**原始** 6 声道以 -15 dB 汇总到 LFE/sub，再过 3 抽头 shaping。
   这是 bass management，不是低音提升或谐波合成。
2. 限幅链：峰值包络→20log10→软拐点限幅→10-tap 增益平滑→三次增益映射→
   乘回音频。

两条链都未产生 Missing Fundamental 的 2/3 次谐波；PDF 描述更接近 G6 硬件
DSP 的 SBX Bass。另有一条独立的 `CBassMgmtEfxMod`（vftable `0x18018cb38`）
是 THX 低音管理：解析 `bass` 配置文件的 crossover/boostlevel/virtuallevel，
用双二阶交叉滤波和 128 帧参数淡化做分频/路由；同样不是谐波合成。

`KSUSBSPI64.dll` 里还有 THX 参数搬运函数：

- `FUN_0044aa48` 读 `THXXBass_Amount`
- `FUN_0044b164` 处理 `THXXBass_isEnabled`
- `FUN_0044a5d8` 读 `THXXBass_CrossOverFrequency`
- `FUN_004580fc` / `FUN_00457ae8` 处理 `THXBassMgt_*`

这些函数做的是属性存储 → 效果对象 vtable 调用（`+0x18/+0x20/+0x30/+0x38/
+0x48/+0x50`），不是采样循环本身。

## Dialog+ 的代码级证据

`KSUSBAPO64.dll` 里**没有** `Dialog`/`Dialogue` 字符串，因此它不太可能是
这个 APO 的独立节点。`KSUSBSPI64.dll` 里有：

- `THXDialogueAdvantage_Amount` → `FUN_0044c1ac`
- `THXDialogueAdvantage_isEnabled` → `FUN_0044c62c`

同样是参数搬运/属性存储，未看到 2–4 kHz 带通或噪声抑制的采样循环。
托管层有 `_etParamDialogPlus`（Enable/Strength/Mode），只能证明参数存在。

Dialog+ 的实际 DSP 可能在：
- 另一个 THX APO/模块（`KSUSBSPI64.dll` 内部未带 RTTI 的效果类）；
- G6 板载 SB-Axx1 固件；
- 或由硬件 DSP 按设备预设执行，Windows 侧只下发参数。

没有 G6 固件/实机回环，无法确认。

## 参考实现（不是逆向还原）

### Bass：Missing Fundamental

`reference/g6_bass.c/h`：

1. 低通提取 cutoff 以下低频（默认 120 Hz，可调 40–400 Hz）。
2. 生成 2 次/3 次项：`h2 = low²`、`h3 = low³`，混合 `0.5*h2 + 0.25*h3`。
3. 高通（1.5×cutoff）去掉直流/基频，2 kHz 低通柔化。
4. `y = x + strength * harmonic`，strength 0..100。

实测（48 kHz，60 Hz 正弦，strength 100）：

| 分量 | 相对 60 Hz |
|---|---|
| 120 Hz | -26.4 dB |
| 180 Hz | -39.9 dB |
| 240 Hz | -107.6 dB |
| 输出峰值 | 0.524（输入 0.5，未削顶） |

另有一个历史实验模式 `--bass-mode apo`（2026-11-19 已标记为 legacy）：

- 它把 `cc560` 的 10-tap FIR 和 `cb5e0` 的三次多项式直接作用在音频上：
  `[0,0,.05,.1,.15,.2,.2,.15,.1,.05]`、8× pre-gain、三次多项式、
  20 Hz DC 阻断、输出增益 `0.17782794`。
- 后续反编译确认这两个函数作用在**增益包络**上（dB→线性增益），
  所以这个模式不是 APO 的逐位实现，只是历史对比；默认仍用 reference。
- 60 Hz / strength 100 实测：180 Hz -20.8 dB、300 Hz -30.5 dB、
  120 Hz -45.8 dB，峰值 0.464。

### Dialog+：2–4 kHz + 轻量维纳门

`reference/g6_dialog.c/h`：

1. 2 kHz 高通 + 4 kHz 低通提取语音带。
2. 带内包络（5 ms 攻击 / 100 ms 释放）+ 慢噪声底跟踪。
3. `snr = env / noise`，`gain = snr/(snr+1)`。
4. `y = x + strength * 2 * gain * band`。

实测（1 kHz + 3 kHz 等幅，strength 100）：

| 分量 | 相对 1 kHz |
|---|---|
| 3 kHz | +5.8 dB |
| 2 kHz | 极低 |
| 4 kHz | 极低 |
| 输出峰值 | 0.674（输入 0.4，未削顶） |

## 接入位置

链路顺序：

```text
EQ -> Crystalizer -> Bass -> Dialog+ -> Surround(6ch) -> SVM
```

- `sbc-dsp`：
  `--bass N --bass-cutoff HZ --dialog N`
- `sbc-live`：
  Bass 复选框 + 强度/截止频率；Dialog+ 复选框 + 强度；运行中实时生效。
- 实时自测（UU 虚拟对，5 秒，Bass 100 + Dialog 100）：
  buffer 48 ms，underrun 0、overrun 0。

## 边界

- 这两个模块是**参考近似**，不是 G6 硬件/APO 的逐位实现。
- Bass 的 PDF 描述是 Missing Fundamental；APO 的 `CBassManagementEfxMod`
  是限幅/动态链，`CBassBoost` 的提升内核仍未定位，两者是否同一算法未证明。
- Dialog+ 的 DSP 位置未找到；PDF 的“维纳滤波类”只是方向。已排除 SPI
  参数处理器和 `CVocalSuppressorEfxMod`（后者是 4 段滤波器组，未证实）。
- 截止频率、谐波比例、门限/时间常数都是可调参考值，不是设备实测值。
- 需要 G6 回环或固件提取，才能把参考实现和真实算法对齐。

## 下一步

1. `CBassManagement` 的分频/路由与限幅都已到函数级；若要确认 `CBassBoost`
   是否还有独立的低频提升，需继续追 `CBassBoostMixer*` 的非虚调用点。
2. 还原 `FUN_1800cc310` 的 `+0x28/+0x2c/+0x1bf0` 初值和 `CBassManager`
   构造路径，补齐限幅器参数。
3. Dialog：主机 APO/SPI 已无线索；G6 固件提取不现实，只能等实机回环或
   Creative 公开资料。
