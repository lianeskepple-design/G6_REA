# Smart Volume（CPlugAGCCompressor）代码级移植

2026-11-21。对象：`KSUSBAPO64.dll` 1.2.109.0（`C:\Windows\System32`，MD5 `62e80aeedded9eeb9bfcc5f5bb52b2d7`）。
本文件记录 `reference/g6_svm.c` 这一版实现的全部依据，以及它推翻的旧结论。

## 一句话结论

Smart Volume 是**单检测器、全带**的 AGC/压缩器，不是 PDF 说的多频段 AGC + 分频段 DRC + 峰值限幅。
这一版按反编译逐行重写：20 ms 前瞻、两抽头延迟交叉淡化、三级平滑、启用时 10^1.1（+22 dB）补偿增益。
旧实现只有一级平滑、30 样本前瞻、一级增益，且 `set_level` 的 `(level_db-12)/10` 映射在 DLL 里不存在。

## 1. 证据锚点

| 角色 | 地址 | 说明 |
|---|---|---|
| 构造 | `0x180092810` | 写常量与默认值，随后调用 reset 与参数更新 |
| 复位 | `0x180092bc0` | 清 2×0x2590 延迟环、`+0x4bc4`/三个增益置 1.0 |
| 检测器系数 | `0x180092a50` | 13 组 (k1,k2)，按采样率取 |
| 参数更新 | `0x180092da0` | 由采样率算 5 个时间常数与延迟 |
| 样本循环 | `0x180092f30` | 检测→增益律→三级平滑→两抽头输出 |
| 启用/旁路 | `0x180093420` | `+0x4b9c` wet、`+0x4b90` makeup 交叉淡化 |
| 改延迟 | `0x1800935c0` | 钳位后写入当前淡出的那个抽头 |
| 改起音/释放 | `0x180093520` / `0x1800936c0` | 只影响 `+0x4b84` / `+0x4b88` |
| log10 | `0x1800dc0b0` | 向量化 log10f |
| 10^x | `0x1800d96d0` | 以 10 为底 |
| CPlug 基类构造 | `0x180096fe0` | 拷贝源格式、协商、镜像到 `+0x34` |
| 格式比较 | `0x180098120` | 4 个 u32 任一不等即返回 true |
| 内部格式解码 | `0x180098470` | `{率, 类型, 声道, 掩码}` → `WAVEFORMATEXTENSIBLE` |
| 样本类型表 | `0x180198f28` | u16[6] = 0,1,2,3,4,4 字节/样本 |

`0x1800dc0b0` 是 log10 而不是 ln，判据有两条：
1. 它引用的常量 `0x1801a0a00 = 0x3ede5bd9 = 0.43429449`，即 log10(e)；
2. 用 log10 解释时，`+0x4b98 = -6.2`（= -62 dBFS 能量）与 `10^1.1 = +22 dB` 都是整数 dB 设计值，用 ln 解释则不是。

## 2. 算法

常量（`0x180092810` 写入）：

```c
+0x4b98 = -6.2f          /* 阈值，log10(能量) */
+0x4b8c =  0.4f          /* 增益律斜率，1-0.4 = 0.6 的压缩指数 */
+0x4b9c =  1.0f          /* wet */
+0x4b90 =  powf(10, 1.1) /* makeup = 12.5893 */
+0x4bf8 =  0.004f        /* attack 上限秒 */
+0x4bf4 =  8.0f          /* release 秒 */
+0x4bf0 =  0.02f         /* 默认前瞻秒 */
```

由采样率推导（`0x180092da0`，全部为 `1 - 10^(-1/(fs*T))` 形式）：

```c
alpha_energy = 1 - 10^(-1/(fs*0.04))                  /* 能量平滑 40 ms      */
alpha_makeup = 1 - 10^(-1/(fs*0.01))                  /* makeup 平滑 10 ms   */
ramp_step    = 1/(fs*0.08)                            /* 交叉淡化 80 ms      */
attack       = 1 - 10^(-1/(fs*0.004*0.46593979))      /* = 1.8638 ms         */
release      = 1 - 10^(-1/(fs*8))
delay        = clamp(seconds*fs, 30, 0.05*fs)         /* 默认 960 @ 48 kHz   */
```

每帧（`0x180092f30`）：

```c
/* 1. 检测：每声道一阶高通 + 能量平滑，取各声道最大值 */
f      = (x - prev_x)*k1 - k2*prev_f;
energy = alpha_energy*f*f + (1-alpha_energy)*energy;
det    = max(det, energy);

/* 2. 增益律：阈值以下增益 1 */
lg = log10(det);
target = (lg < -6.2) ? 1 : 10^((lg + 6.2)*0.4 - 6.2 - lg)   /* = 10^(-0.6*(lg+6.2)) */

/* 3. 三级平滑（注意 +0x4be8 跟的是上一帧的 gain1） */
coeff = (gain1 <= target) ? release : attack;      /* 下降用 attack，上升用 release */
gain1 = (1-coeff)*gain1 + coeff*target;
blend = ((wet*gain2 + 1) - wet) * makeup;
applied = mix_gain;                                 /* 输出用上一帧的值 */
gain2 = (1-attack)*gain2 + attack*gain1_previous;
mix_gain = (blend - mix_gain)*alpha_makeup + mix_gain;

/* 4. 两抽头延迟交叉淡化 + 补偿增益 */
out = ((1-crossfade)*ring[idx(delay_b)] + crossfade*ring[idx(delay_a)]) * applied;

/* 5. 交叉淡化推进（0x4bb8 上升 / 0x4bb9 下降） */
```

延迟环每声道 2400 个 float（`0x960`，对象内步长 `0x2590`），写指针先写后自增；读索引用
`(write < taps) ? write - taps + 2400 : write - taps`，所以实际延迟是 `taps - 1` 帧。

`0x180093420` 的旁路不是硬开关：`wet = 0`、`makeup = 1`，延迟改成 30 样本，然后同样走交叉淡化；
`0x1800935c0` 永远写"当前权重为 0 的那个抽头"，再反向淡过去。

## 3. 48 kHz 下的实测常量（`svm_out.txt`）

| 名称 | 值 | 位型 |
|---|---|---|
| k1 / k2 | 0.987100005 / -0.97420001 | `3f7cb296` / `bf79652c` |
| alpha_energy | 0.0011985302 | `3a9d1800` |
| alpha_makeup | 0.00478553772 | `3b9cd000` |
| ramp_step | 0.00026041668 | `39888889` |
| attack | 0.0254101753 | `3cd02900` |
| release | 6.02006912e-06 | `36ca0000` |
| threshold | -6.19999981 | `c0c66666` |
| ratio | 0.400000006 | `3ecccccd` |
| makeup | 12.5892544 | `41496d96` |
| delay_a / delay_b | 2400 / 960 | `45160000` / `44700000` |

## 4. 与旧实现的差异

| 项 | 旧 | 新（本版） |
|---|---|---|
| 前瞻 | 30 样本固定 | 20 ms 默认（48 kHz 下 960），30 样本下限，0.05·fs 上限 |
| 输出 | 单抽头 + 一级增益 | 两抽头交叉淡化 + `applied` 一帧延迟 |
| 平滑 | 一级（attack/release） | 三级（gain1、attack 平滑 gain2、10 ms makeup 平滑） |
| 补偿增益 | 无 | 启用时 10^1.1 = +22 dB |
| 旁路 | 跳过处理 | `wet=0 / makeup=1 / 30 样本延迟` 的交叉淡化 |
| `set_level` | `threshold=(level_db-12)/10`，声称来自 `0x180093710` | `0x180093710` **不是函数**；DLL 只在构造里写 `+0x4b98`，没有任何电平旋钮。旧映射保留为 legacy，新增 `g6_svm_set_threshold_dbfs` |
| 模式曲线 | 文档写"仍未细化" | 不存在：DLL 没有 Normal/Loud/Night 曲线表 |

## 5. 与 PDF「多频段 AGC + DRC + 限幅」的对照

- 检测器：**每声道一个**一阶高通 + 能量平滑，然后跨声道取最大值。没有分频段、没有 per-band 检测。
- 增益律：单一 log 域线性压缩，阈值固定 -6.2（log10 能量），斜率 0.6。
- 限幅：只有 `target` 的上界 1.0，没有独立 limiter，也没有峰值保护。
- 结论：PDF 的多频段描述与主机 APO 的实现不符。要么 G6 硬件里另有一套，要么 PDF 是按产品行为推断的。
  在拿到实机回环之前，本项目只能声称复刻了 APO 的这一套。

## 6. 顺带更正：Crystalizer 没有 `+0xF18` 输出级

- 扫描 `.text` 全量反汇编，位移 `0xf18` **0 次**命中（`G6Probe scan 0xf14 0xf18 0xf1c 0xf20 0xf24`）。
- `0x18008E920` 的真实结构是：每声道三阶滤波状态 `this+0xe50+ch*0x18`（vtable `+0x2b8`）、
  二阶状态 `this+0xe98+ch*0x28`（vtable `+0x2a8`），最后是 ramp 混合（vtable `+0x180`）。
  旧文档把"`0x18008EC00` 的 `+0x2A8`"读成了对象偏移，其实那是 **vtable 槽位**。
- 因此 `g6_crystalizer.c` 里的 `output_filter` / `g6_crystalizer_set_output_filter` 是幻影级，已删除；
  探针改为输出 `output_stage none`，参考脚本据此校验，防止再把结论写回去。

## 7. 验证（2026-11-21）

| 检查 | 结果 |
|---|---|
| `tools/apo-eq-coeff/verify_svm.cmd` | PASS（14 个常量位级一致、6144 帧增益轨迹、交叉淡化、旁路） |
| 期望的 0.08 s 交叉淡化 | 在第 3840 帧精确落到 0 |
| `tools/apo-eq-coeff/verify_crystalizer.cmd` | PASS（`output_stage none`） |
| `cargo test` | 31 passed（22 + 6 + 3） |
| `sbc-dsp` 端到端（`imp.wav`，`--svm-level -20`） | 48000 Hz / 2ch / 131072 帧，peak 0.25，rms 0.000488，全有限 |
| `--svm-delay-ms 5 / 50` | 均正常输出，5 ms 被钳到 30 样本 |

行为抽样（`svm_out.txt`，1 kHz 正弦）：

| 段 | 输入幅度 | gain1 | 输出增益（含 makeup） |
|---|---|---|---|
| 0–2048 | 0.01 | 1.0 → 0.0786 | 5.22 → 1.02 |
| 2048–4096 | 0.5 | 0.0773 → 0.0007 | 1.02 → 0.0094 |
| 4096+ | 0.01 | 0.0007（8 s 释放，几乎不动） | 0.0091 |

即：小声被抬到约 -40 dBFS，大声被压到约 -49 dBFS 量级——是"夜间模式"式的高压缩，不是线性归一化。

## 8. CPlug 基类构造与内部格式协商（`0x180096fe0`）

`0x180096fe0` 是 `CPlug` 的基类构造，不是 AGC 专属 [E1]。

```
FUN_180096fe0(CPlug *this, CSource *source, uint minRate, uint maxRate,
              uint sampleType, uint channels)
```

### 8.1 基类构造的动作 [E1]

1. 先把 `this` 临时挂上 `CStreamingSink::vftable`（`0x18018b758`，`0x18009700b`），再覆盖为
   `CPlug::vftable`（`0x180196c48`）；`this+0x20` 挂第二套接口表（`0x180196c78`）[E8]。
2. `this+8 = source`，`source+8 = this`，双向回指。
3. `this+0x10..0x1c = source+0x14..0x20`，即源端当前的 4 dword 内部格式。
4. `this+0x48 = minRate`、`+0x4c = maxRate`、`+0x50 = sampleType`、`+0x54 = channels`。
5. 执行 8.3 的协商，然后把 `this+0x10..0x1c` 整份镜像到 `this+0x34..0x40`。

### 8.2 内部格式的 4 个字段 [E7]

`0x180098470` 把这个 4 dword 结构翻译成 `WAVEFORMATEXTENSIBLE`，字段顺序由它确定：

| 偏移 | 字段 | 去向 |
|---|---|---|
| `+0x10` | 采样率 | `nSamplesPerSec` |
| `+0x14` | 样本类型 | `SubFormat` + `wBitsPerSample`（查表） |
| `+0x18` | 声道数 | `nChannels`、`nBlockAlign`、`nAvgBytesPerSec` |
| `+0x1c` | 声道掩码 | `dwChannelMask` |

样本类型表 `0x180198f28` 是 u16[6]：`0,1,2,3,4,4` 字节/样本。
1–4 配 PCM GUID（`0x180191558` = `{00000001-0000-0010-8000-00aa00389b71}`），
5 配 IEEE float GUID（`0x180185498` = `{00000003-…}`），字节数同为 4，即 float32 [E7]。

因此 `+0x18` 是声道数，不是旧记的 `+0x14`。DSP 侧也这么读 [E6]：

- 输入循环上限 `*(uint*)(this+0x18)`，输出循环上限 `*(uint*)(this+0x3c)`；
- `+0x1c` / `+0x40` 是 `& 0x7fffffff` 的声道掩码，0 表示不限。

### 8.3 协商算法 [E3][E4]

```c
requested.rate     = (minRate <= src.rate) ? min(src.rate, maxRate) : minRate;
requested.type     = sampleType;                  /* 恒为请求值，不钳位 */
requested.channels = min(channels, src.channels);
requested.mask     = 0;
if (formatDiffers(this + 0x10, &requested) && source->SetFormat(&requested) >= 0)
    this->format = requested;                     /* 否则保留源格式 */
```

`0x180098120` 是「任一字段不等」判定：4 个 u32 全等返回 false，否则 true [E4]。
不要把返回值读成「相等」。

### 8.4 两个调用点 [E2][E8]

| 调用点 | 类 | minRate | maxRate | sampleType | channels |
|---|---|---|---|---|---|
| `0x180091b1f` | CPlugReverb | 22050 | 96000 | 5 | 1 |
| `0x180092842` | CPlugAGCCompressor | 200 | 48000 | 5 | 2 |

AGC 的实参来自 `0x180092820`：`MOV R8D,0xc8`、`MOV R9D,0xbb80`、
`[RSP+0x20]=5`、`[RSP+0x28]=2` [E8]。全镜像只有这两处 `CALL 0x180096fe0` [E2]。

### 8.5 本机默认路径不触发协商 [E5]

`0x1800643b0` 建的 `CDummySource` 写入 `{48000, 5, 2, 0}`。AGC 的请求是
`{48000, 5, min(2,2)=2, 0}`，4 个字段全等 → `0x180098120` 返回 false → 不调用 SetFormat，
`this+0x10..0x1c` 保持源格式。

Reverb 对同一个 `CDummySource` 的请求是 `{48000, 5, 1, 0}`，第三字段 1≠2 →
会调 SetFormat 请求单声道。

### 8.6 边界

- [已验证] `minRate` 只在 `src.rate < minRate` 时成为请求值；DSP 端没有对应下限。
  AGC 的 200 在正常源上不可达（源率均 ≥ 8 kHz），业务理由未取证 [E3]。
- [已验证] 两个已知调用点的 `sampleType` 都是 5，内部链路按 float32 协商 [E2]。
- [未验证] `sampleType = 5` 遇到 PCM 源时 `SetFormat` 是否成功、重采样归谁，未测。

## 9. 未完成

- 输入声道数（`+0x18`）与输出声道数（`+0x3c`）在构造时被镜像成同一份格式；
  单独改写 `+0x3c` 的代码路径尚未找到，本实现按相等处理。
- `minRate = 200`（AGC）与 `22050`（Reverb）只参与格式协商，业务理由未取证。
- 源为 PCM 或源率低于下限时协商的成败与后续重采样归属，未测。
- 实机 Smart Volume 听感与 APO 是否一致，仍需 G6 回环。
