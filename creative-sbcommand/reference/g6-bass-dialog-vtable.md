# Bass / Dialog+ vtable 级逆向记录

2026-11-19。目标：
- `KSUSBAPO64.dll`（x64，ImageBase `0x180000000`）的 `CBassBoost` / `CBassManager` / `CBassManagementEfxMod`。
- `KSUSBSPI64.dll`（x64，ImageBase `0x400000`）的 `THXDialogueAdvantage_*` 参数处理器。

结论先说：
- **Bass**：已到处理函数级。`CBassManagementEfxMod` 的 init/process vtable、`CBassManager` 的构造/process、FIR/增益映射/限幅/混合子函数都已定位；FIR 与增益映射作用于增益包络，另有 `FUN_180089780` 的分频/低音路由核心。两条链都不是 Missing Fundamental 谐波合成。
  2026-11-21 补完：两个 mixer 都是内嵌 `CHspIIR2State`（`+0x08`）与 `CHspLimiter1State`（`+0x30`）
  的 0x48 复合对象；`CBassBoost` 只是 `CBassManager+0x60` 的 12 × 0x28 状态槽，没有采样循环（§8）。
- **Dialog+**：只到“参数处理器 vtable”。SPI 里的 `THXDialogueAdvantage_Amount/isEnabled` 函数是属性读写 + vtable 调用，没有采样循环；APO 的 `CVocalSuppressorEfxMod` 是 4 段滤波器组候选，不是 Dialog+；实际 DSP 仍未找到。

## 1. Bass：RTTI 与 vtable

Python 解析 RTTI TypeDescriptor/COL/vftable：

| 类 | TypeDescriptor | vftable | slot0 |
|---|---|---|---|
| `CBassBoost` | `0x180206198` | `0x1801917e0` | `0x1800896c0` |
| `CHspBassBoostState` | `0x180206168` | `0x1801917d0` | `0x180089750` |
| `CBassBoostMixerBase` | `0x180205c08` | `0x1801886d0` | `0x18003dd20` |
| `CBassBoostMixerSmallSpeakers` | `0x180205bd0` | `0x1801886e0` | `0x18003dd60` |
| `CBassBoostMixerLFE` | `0x180205ce8` | `0x180188bd0` | `0x180040b10` |
| `CBassManager` | `0x1802061c0` | `0x1801917f0` | `0x180089710` |

`CBassBoost` 等辅助类的 vftable 只有一个析构槽，处理逻辑不是虚函数；
实际入口在 `CBassManagementEfxMod` / `CBassMgmtEfxMod` 的 vtable。

## 2. Bass：EfxMod vtable 槽位

`CBassManagementEfxMod` vftable `0x180188560`：

| 槽 | 函数 | 大小 | 角色（由反编译判断） |
|---|---|---|---|
| +0x18 | `FUN_18003e420` | 1036 | init/属性：分配 mixer/IIR2/limiter/manager，读 9 个属性 |
| +0x40 | `FUN_18003de90` | 769 | 块处理包装：调用 `FUN_180088ff0` + `FUN_1800cc050` |
| +0x68 | `FUN_18003ee80` | 256 | 参数/状态 |
| +0x80 | `FUN_18003e1a0` | 228 | 参数/状态 |
| +0x88 | `FUN_18003e290` | 228 | 参数/状态 |
| +0xa0 | `FUN_18000e3c0` | 280 | 通用属性 |
| +0xd8 | `FUN_18003ec30` | 564 | 参数/状态 |
| +0xe0 | `FUN_18003ef90` | 468 | 参数/状态 |

`CBassMgmtEfxMod` vftable `0x18018cb38`：

| 槽 | 函数 | 大小 |
|---|---|---|
| +0x18 | `FUN_180066d40` | 1721 |
| +0x40 | `FUN_1800669a0` | 607 |
| +0x48 | `FUN_180068200` | 1026 |
| +0x68 | `FUN_180067be0` | 1380 |
| +0xf8 | `FUN_180067440` | 1092 |

`CBassManager` vftable `0x1801917f0`：

| 槽 | 函数 | 大小 |
|---|---|---|
| +0x00 | `FUN_180089710` | 53 |
| +0x08 | `FUN_180089e40` | 76 |
| +0x10 | `FUN_180089db0` | 76 |
| +0x20 | `FUN_180089e00` | 54 |
| +0x28 | `FUN_180089b80` | 122 |

`CBassManager` 构造 `FUN_1800894e0`：设置 `CStreamingSink`/`CBassManager` vftable，
在 `+0x60` 构造 `CBassBoost`，清零 `+0x248` 的 0xC0 字节状态，并写
`+0x328 = -1`、`+0x32c = 0`、`+0x330 = 100`、`+0x334 = 1.0f`、`+0x338 = 0`、
`+0x33c = 100`、`+0x340 = 1.0f`（逐字段见 §8.6）。

## 3. Bass：处理链

`FUN_18003de90`（块包装）：
1. 调 `(*(param_1+8))+0x40`（上游块处理）。
2. 有效时复制块描述 `local_88`，调 `FUN_180088ff0` 做通道路由/混合：
   - `+0x1F0` 加权累加、`+0x2A8` 二阶、`+0x450`、`+0x158`、`+0x1F8` 叠加。
3. 单声道/多声道块循环，调 `FUN_1800cc050`（BassManager process）。
4. 调 `FUN_1800cc050` 内部再分发：
   - `FUN_1800cb830`（逐声道 max(abs) 包络；用 `0x7fffffff` 掩码）
   - `FUN_1800cb4a0`（max > 1.0 时置 clip 标志）
   - `FUN_1800cc170`（对数/dB 转换：`FUN_1800e32dc` + 多项式，供限幅器用）
   - `FUN_1800cc310`（限幅器：阈值/攻击/释放，逐样本）
   - `FUN_1800cc560`（10 抽头 FIR）
   - `FUN_1800cb5e0`（三次增益映射，dB 包络 → 线性增益）
   - `FUN_1800cb9e0`（交叉淡化/多通道混合）

`FUN_1800cc050` 先算 clip 标志：未 clip 时只调用 `FUN_1800cb750`
旋转延迟线；clip 时才走 `cc170/cc310/cc560/cb5e0/cb9e0` 分支。

**2026-11-19 修正（重要）**：`cc560` 的 10-tap FIR 和 `cb5e0` 的三次
整形**作用在增益包络上，不是音频样本**。`cc170` 把 `cb830/cb4a0` 算出的
峰值包络转成 dB，`cc310` 在 dB 域做软拐点限幅，`cc560` 平滑这条增益
曲线，`cb5e0` 把平滑后的 dB 值映射成线性增益，`cb9e0` 再把增益乘回音频
并做交叉淡化/延迟参考混合。因此 `CBassManagementEfxMod` 是**限幅器/
动态处理链**，不是缺失基频谐波合成器；Bass 的实际提升在 `CBassBoost`
mixer 或硬件 DSP，不能拿 `cc560/cb5e0` 当音频谐波算法。

### Bass 管理的混音增益

`FUN_180088ff0`（`CBassManagementEfxMod` 块包装调用的路由函数）通过全局
mixer 对象 `PTR_DAT_1801f1e80` 的 vtable 槽做通道混合：

| 槽 | 调用 | 增益 |
|---|---|---|
| +0x1F0 | 每个被选源声道 → 目标缓冲 | `DAT_180191738` = 0.1778279394（约 -15 dB） |
| +0x2A8 | 目标缓冲 → `param_1+8` | — |
| +0x450 | 目标缓冲 → `param_1+0x30` | — |
| +0x158 | 目标缓冲 → 目标缓冲 | `DAT_180191740` = 3.3496544361（约 +10.5 dB） |
| +0x1F8 | 低音/LFE 源声道 → 目标缓冲 | — |

`CBassManager` 构造 `FUN_1800894e0` 要求 `param_1+0x14 == 5`（5 声道），
否则返回 `0xC00000BB`；这进一步说明它是**低音管理/路由**（小音箱 → sub/LFE），
不是谐波合成。真正的 SBX Bass（Missing Fundamental）在主机 APO 里未定位，
可能在 `CBassBoost` 的非虚 mixer 代码或 G6 硬件 DSP。

### Bass 管理核心 `FUN_180089780`

`CBassManager` 的 vtable `+0x28`（`FUN_180089b80`）在非“直通”分支调用
`FUN_180088d80` → `FUN_180089780`，这是**分频/低音路由**的采样循环：

- 外层按帧（`param_2+4` 是帧数），内层固定 6 个源声道（`iVar9=2..7`，
  跳过 4/5 的 LFE 相关声道）。
- 每个声道过一个一阶 IIR（状态是“上一输入 + 上一输出”）：
  ```text
  y[n] = c*(x[n-1] - x[n] - 1e-8) - a2*y[n-1]
  ```
  其中 `c = +0x308 = +0x30c = -2/(k+2)`，`a2 = +0x310 = (k-2)/(k+2)`，
  `k = 3π*cutoff/sample_rate`（`FUN_180089ea0` 计算）。
  传递函数 `H(z) = c*(z^-1 - 1)/(1 + a2*z^-1)` 在 DC 为 0、在 Nyquist
  非零，是**高通**，不是低通。
- 同一循环把高通后的信号写回各声道（小音箱去低频），而
  `fVar16` 累加的是**原始**声道样本乘 `DAT_180191738 = 0.17782794`
  （-15 dB），再经过 3 抽头/二阶 shaping（`+0x314..+0x324`），
  最后乘 `+0x5c` 输出到 LFE/sub。
- 因此它是 **bass management**：卫星声道高通 + 原始低频汇总到 sub，
  不是“提升各声道低频”，也不是谐波合成。

结论：主机 APO 的 `CBassManagementEfxMod` 是**分频/低音路由 + 限幅**，
不是 Missing Fundamental 谐波合成；`FUN_180089780` 是分频/路由核心，
`cc560/cb5e0` 是限幅器的增益包络处理。两条链都已到函数级，但都不是
PDF 的“缺失基频”算法。

### 另一条：`CBassMgmtEfxMod`（THX bass management）

vftable `0x18018cb38` 是 THX 低音管理，不是 SBX Bass：

- `+0x18` `FUN_180066d40`：读 9 组属性（交叉频率/boost/virtual level 等），
  分配 0x140 字节状态。
- `+0x40` `FUN_1800669a0`：块处理，按 128 帧调 `FUN_1800a9670`。
- `+0x48` `FUN_180068200`：按输出声道布局（0x3f/0xff/0x60f/0x63f 等）
  选择交叉/混合模式，配置 `FUN_1800a9370`。
- `+0x68` `FUN_180067be0`：参数提交（模式 0..3）。
- `FUN_180067890`：解析外部 `bass` 配置文件，读 `crossover`、`boostlevel`、
  `virtuallevel` 等字段。
- `FUN_1800a9670`：逐帧 DSP，调用 `FUN_1800c6d30`/`FUN_1800c7860`
  （双二阶/交叉滤波）和 `FUN_1800c7b70`（2 抽头 FIR），带 128 帧
  参数交叉淡化。

它做的是**分频/低音路由/虚拟低音电平**，仍然没有 2/3 次谐波合成。
因此主机 APO 里两条 Bass 路径（SBX bass redirect + THX bass management）
都不是 Missing Fundamental；SBX Bass 应在 G6 硬件 DSP。

### 10 抽头 FIR（增益包络平滑）

`FUN_1800cc560` 从 `DAT_1801f7a28` 取系数，逐样本卷积 `cc310` 输出的
dB 增益包络（缓冲区首地址前移 0x1c 字节，前 7 个样本是历史）：

```text
[0.0, 0.0, 0.05, 0.1, 0.15, 0.2, 0.2, 0.15, 0.1, 0.05]
```

和 = 1.0，是一个对称低通 FIR，用来平滑增益变化、避免限幅器“抽气”。
反编译里用 `DAT_1801f7a30`/`DAT_1801f7a34` 偏移访问，实际表起点是
`0x1801F7A28`；10 个 tap 之后是相邻数据，不是系数。

### 增益映射（不是波形整形）

`FUN_1800cb5e0` 的输入是上一步平滑后的 dB 增益包络，输出是线性增益：

```text
x = env_db * 0.166096404
frac = x - round_to_nearest_int(x)
g = (((frac * 0.0397112630 + 0.2327848822) * frac + 0.6930736303) * frac
     + 1.0) - 1.0e-6
g = FUN_1800e33b8(g)      // double 指数调整辅助（_sptype/_set_exp，近似 ldexp）
gain = g
```

`frac` 落在 [-0.5, 0.5]，`g` 约在 0.66..1.4（约 ±3 dB），即限幅器的
增益修正曲线。`FUN_1800e33b8` 做 double 的指数/尾数调整，调用处指数
参数为 0 或被优化掉，因此结果基本就是上面的三次多项式。

`FUN_1800cc170` 是线性幅度到 dBFS 的转换：`FUN_1800e32dc`（近似 frexp）
取出尾数 `m` 和指数 `e`，再用

```text
p(m) = ((m*1.3084441423 - 4.3831000328)*m + 6.2848725319)*m
       - 3.2102167606 + 3.99999999e-06
db   = (p(m) + e) * -6.0205998421
```

`p(m)` 在 [0.5,1) 上就是 `log2(m)` 的三次逼近，`-6.0206 = -20*log10(2)`，
所以输出是 `20*log10(x)`（dBFS，非正数）。

### dB 域限幅器

`FUN_1800cc310(samples, count, state)`，`state` 偏移：

| 偏移 | 含义（按用法） |
|---|---|
| `+0x28` | 攻击/释放系数 `attack` |
| `+0x2c` | 拐点后的斜率 `ratio` |
| `+0x34` | 上一采样输出 `s`（dB） |
| `+0x1bf0` | 拐点宽度 `knee`（dB，非正） |

逐样本：

```text
d = env_db - s
if d > 0:
    if d + knee <= 0: g = (d + 0.001) * attack
    else:             g = ((d + knee) * ratio + 1.0) * attack * (d + 0.001)
    s = s + g
    if s > 0: s = 0
env_db = s
```

即软拐点前馈限幅器；`0.001` 是偏置，`+0x1bf0` 为负的拐点宽度。
`+0x28/+0x2c/+0x1bf0` 的初值已还原：`+0x2c = 0.5`、`+0x1bf0 = -4.5 dB`，
`+0x28 = 1 - exp(-2432 / (rate * 250 / [2]))`，`[2]` 在 ≥88.2 kHz 时取 16、
否则取 8（§8.2）。

### 其它常量

| 常量 | 值 | 用途 |
|---|---|---|
| `DAT_18019E19C` | 0.0397112630 | 增益曲线三次项 |
| `DAT_18019E1A0` | 0.1660964042 | dB 缩放（1/6.0206） |
| `DAT_18019E1A4` | 0.2327848822 | 增益曲线二次项 |
| `DAT_18019E1A8` | 0.6930736303 | 增益曲线一次项 |
| `DAT_180199148` | 1.0e-6 | 偏置 |
| `DAT_1801865CC` | 1.0 | 单位 |
| `DAT_18019E198` | 3.99999999e-06 | log2 多项式附加常数 |
| `DAT_18019E1AC` | 1.3084441423 | log2 三次项 |
| `DAT_18019E1B0` | 3.2102167606 | log2 常数项（减） |
| `DAT_18019E1B4` | 4.3831000328 | log2 二次项（减） |
| `DAT_18019E1B8` | 6.2848725319 | log2 一次项 |
| `DAT_18019E1BC` | -6.0205998421 | dB 换算（-20*log10(2)） |
| `DAT_180191738` | 0.1778279394 | 路由增益（约 -15 dB） |
| `DAT_180191740` | 3.3496544361 | 路由/混合增益 |

`FUN_18003e420` 从 `DAT_1801775d0` 复制 9 条 **属性描述符** 到
`param_1[0xd]`（0xB4 字节，`param_1[0xe] = 9`）。每条 0x14 字节，
即 `PROPERTYKEY`：16 字节 GUID + u32 pid。三个 GUID 各出现 pid 0..2：

| GUID | index |
|---|---|
| `{cea7dab2-7e38-4bb6-b32d-ed366a966bf2}` | 0, 1, 2 |
| `{d894c4d0-a6b9-4ca6-bf1d-70429b16394d}` | 0, 1, 2 |
| `{90b7f7e6-7da2-4564-9858-91957a70b552}` | 0, 1, 2 |

它们不是滤波器系数，而是 BassManagement 的属性集；GUID 只出现在
`KSUSBAPO64.dll`，SPI/注册表里没有对应字符串，语义（Amount/Enable/
CrossOver 等）尚未逐个钉死。

## 4. Dialog+：参数处理器 vtable

`KSUSBSPI64.dll`（x64，ImageBase `0x400000`）里，`THXDialogueAdvantage_*`
字符串的代码引用：

| 函数 | 字符串 | 作用 |
|---|---|---|
| `FUN_0044C1AC` | `THXDialogueAdvantage_Amount` | 读 0x28 字节属性块，写 `param_2/3/4`，调用 vtable |
| `FUN_0044C62C` | `THXDialogueAdvantage_isEnabled` | 调 vtable +0x18/+0x20/+0x30/+0x38，设置启用状态 |

这些函数位于 `.text` 内的函数指针表（不是普通 `.rdata`）：

- 0x4078F0 附近：`0x44C488, 0x44C510, 0x44C594, 0x44C1AC, 0x44C404,
  0x44C380, 0x44C62C, ...`
- 0x40F9B0 附近是同一组处理器的另一份表（部分槽为 `0x4644CA` 共享桩）。

已解出的 Dialog 专用方法：

| 函数 | 行为 |
|---|---|
| `FUN_0044C380` | `FUN_00416548` 读属性 ID 0x100000006 |
| `FUN_0044C404` | 写属性 ID 0x200000006 |
| `FUN_0044C488` | 写属性 ID 0x200000005 = 1 |
| `FUN_0044C510` | 写属性 ID 0x200000005 = 0 |
| `FUN_0044C594` | 读属性 ID 0x100000005 |
| `FUN_0044C1AC` | 读 `THXDialogueAdvantage_Amount`，返回 3 个归一化值 |
| `FUN_0044C62C` | 读 `THXDialogueAdvantage_isEnabled`，调 vtable 启用/禁用 |

共享槽：

- `0x4226A8` → `sub rcx,0x10; jmp 0x41B244` → `FUN_00414D10`（通用属性分发）
- `0x429BA8` → `sub rcx,0x10; jmp 0x41B170`
- `0x422A28` → `sub rcx,0x10; jmp 0x41B1CC`（引用计数/释放）
- `0x422F08/0x429E88/0x422AA8` 是 `sub rcx,8` 的 adjustor thunk。

`FUN_00414D10` 只做接口/属性表遍历（按 16 字节 GUID 比较），不含采样循环。
因此 **Dialog+ 的 DSP 不在这些处理器里**；可能在：
- SPI 里未带 RTTI 的另一个效果类；
- `KSUSBDGFX64.dll` / `CTAudEp.dll`；
- G6 板载 SB-Axx1 固件（Windows 只下发参数）。

## 5. 与参考实现的关系

- `reference/g6_bass.c` 现在有两种模式：
  - `mode=0`（默认）：低通 + x²/x³ + 高通 + 平滑的参考近似，对应 PDF 的
    Missing Fundamental 方向，但**不是 APO 的逐位实现**。
  - `mode=1`（实验，已标记为 legacy）：把 `cc560`/`cb5e0` 误当成音频
    FIR/整形器，用 8× pre-gain 校准；**这个理解已被本节修正推翻**，
    结果只保留作历史对比，不应作为 APO 算法依据。
- 修正后的 APO 事实：`CBassManagementEfxMod` 是限幅/动态链，`cc560` 平滑
  增益包络、`cb5e0` 把 dB 包络映射成线性增益；Bass 的提升在
  `CBassBoost` mixer 或硬件 DSP，尚未定位。
- 60 Hz / strength 100 / mode=1 实测：180 Hz -20.8 dB、300 Hz -30.5 dB、
  120 Hz -45.8 dB，峰值 0.464；与 mode=0 的 120 Hz -26.4、180 Hz -39.9 不同。
  这些数字只说明两条链不同，不证明 APO 谐波结构。
- `reference/g6_dialog.c` 仍是 2–4 kHz + 轻量门的参考近似，和 SPI 参数处理器
  没有代码级对应。

## 6. 待办

1. ~~解 `FUN_1800cc170` 的常数~~：已解出 `20*log10` 多项式与 dB 换算。
2. 解 `FUN_1800cb9e0` 的交叉淡化系数（`param_5+0xC`、`+0x1C`、`+0x20`）
   与逐块混合顺序；已确认它把增益乘回音频并维护延迟参考线。
3. ~~还原 `FUN_1800cc310` 的 `+0x28/+0x2c/+0x1bf0` 初值（按采样率/阈值）
   以及 `CBassManager` 构造路径~~：已完成，见 §8.2、§8.6。
4. ~~确认 `CBassBoost` 是否还有独立低频提升，追 `CBassBoostMixer*` 的非虚调用点~~：
   已回答。mixer 在 init 里手写 vtable（§8.3），`CBassBoost` 无内核（§8.4）。
   SBX Bass 若不在 APO，只能等实机/公开资料。
5. Dialog：`CVocalSuppressorEfxMod` 已解到 vtable/属性/引擎级，但它是
   4 段滤波器组，不是 Dialog+；主机 APO/SPI 已无线索，G6 固件提取不现实，
   只能等实机回环或 Creative 公开资料。

## 7. `CVocalSuppressorEfxMod`（Dialog+ 的候选，未证实）

`KSUSBAPO64.dll` 里没有 `Dialog` 类名；唯一与“人声/对白”相关的 THX
效果类是 `CVocalSuppressorEfxMod`，因此把它作为候选记录，但**不能**
据此声称它就是 Dialog+。

- TypeDescriptor `0x180204c48`，COL `0x1801c20d0`，vftable `0x18018de98`。
- vtable 槽位与 Bass 同构：`+0x18` init `FUN_180077b20`，`+0x40`
  process `FUN_180077860`，`+0x48` 参数提交 `FUN_180077ee0`。
- init 读取 8 个属性（同一 GUID 的 index 0..7）：
  GUID `{1ee40950-a46c-45eb-a50c-f1fd995c0166}`，描述符在
  `0x18018dd58..0x18018de10`；这个 GUID 只出现在 APO，SPI 没有对应字符串。
- process `FUN_180077860` 只是把 8 个通道/流指针搬进引擎对象
  `param_1+0x790`，然后调 `FUN_1800b3da0(param_1+0x130, param_1+0x690)`。
- `FUN_1800b3da0` → `FUN_1800c95f0`（比较 8 个 float，返回变化位掩码）→
  `FUN_1800b3a30`（按变化位重算滤波器系数）→ `FUN_1800b35c0`/`b3660`
  （系数/状态准备）→ `FUN_1800b3910`（4 段交叉淡化系数）→
  `FUN_1800b3790`（逐块滤波/混合）。
- `FUN_1800c7b70` 是通用多抽头 FIR/双二阶调度器（case 1 为双二阶，
  case 2..8 为 2..8 抽头 FIR）；`FUN_1800c6050` 设计双二阶系数。
- 结论：这是一个 **4 段可交叉淡化的滤波器组**（约 8 个频率/增益参数），
  更像“人声抑制/卡拉 OK”而不是 2–4 kHz 对白增强 + 维纳门。Dialog+ 的
  实际 DSP 仍未找到。

## 8. 2026-11-21 补完：限幅器常量、mixer 实例化与 `CBassBoost` 定案

纯静态取证（Ghidra 12.1.4，`KSUSBAPO64.dll` 1.2.109.0）。未运行原厂代码，
未改动 `C:\Windows\System32` 下任何文件。

### 8.1 对象图：限幅器是 `CBassManagementEfxMod` 的内联区域

没有独立分配的限幅器对象。所有成员都在同一个 EfxMod 里：

| 成员 | 偏移 | 证据 |
|---|---|---|
| `CBassBoostMixerSmallSpeakers*` | `+0x168` | E5：`FUN_18003e420` 写入；E6：`FUN_18003de90` 作第一实参传给 `FUN_180088ff0` |
| `CBassManager*` | `+0x108` | `FUN_18003e420` 写入；`FUN_18003de90` 经 `[+0x20]+8` 槽调用 |
| 对齐后的临时缓冲 | `+0x160` | `FUN_18003eae0` 分配 `frames*4+0x10` 并对齐 16 |
| 限幅器组件（内联） | `+0x188` | E1：`FUN_1800cbdb0(this+0x188, …)` |
| 块描述/双缓冲 | `+0x1da0`、`+0x1da8` | `FUN_18003de90` 各拷 0x1000 字节，再调 `FUN_1800cc050(this+0x188, this+0x1da0)` |
| 限幅器输出结构 | `+0x1e60` | E1 第三实参；组件内 `+0x1c10` 保存它，`+0x40 = this+0x1e7c` |

E1 `FUN_18003eae0` 是“按块大小预分配”：`param_2 % *(uint*)(this+0x5c) == 0`
才继续，记录块大小、重分配缓冲，然后构造三元素入参并调用配置函数：

```c
local_28 = 1; local_24 = (float)*(uint*)(this + 0x20); local_20 = frames;
FUN_1800cbdb0(this + 0x188, &local_28, this + 0x1e60, *(undefined8*)(this + 0x18));
```

### 8.2 限幅器字段与常量

E2 `FUN_1800cbdb0` 里的断言串给出源码身份：`L"temp - floor(temp) == 0.0f"`、
`L"LimiterRefComponent.c"`、行 `0x331`。

| 偏移 | 写入值 | 语义 |
|---|---|---|
| `+0x00` | 采样率（float） | `param_2[1]` |
| `+0x08` | `8` / `16` | 归一化除数 `[2]`；≥88.2 kHz 取 16 |
| `+0x0c` | `1.0f / [2]` | — |
| `+0x10` | `8` | 未解 |
| `+0x14` | 输入码（本路径恒为 1） | `param_2[0]` |
| `+0x18` | 映射码：1→2、2→4、3→6、4→7、5→8 | 阶梯表 |
| `+0x1c` | `72` / `144` | 每声道状态跨距（float）；≥88.2 kHz 取 144 |
| `+0x20` | `floor(frames / [2])` | 断言要求整除 |
| `+0x24` | `9` | 后随 9 个 float 的计数 |
| `+0x28` | `1 - exp(-2432 / (rate * 250 / [2]))` | 攻击系数 |
| `+0x2c` | `0.5` | 拐点后斜率 |
| `+0x34` | `0.0` | 上一采样输出 |
| `+0x38` | `1.0` | 门控基准（`FUN_1800cc050` 用 `+0x38 < 1.0` 决定是否继续限幅） |
| `+0x1bec` | `250.0` | 未解（本轮未定位读取者） |
| `+0x1bf0` | `-4.5` | 拐点宽度（dB） |
| `+0x1bf4` | `1`（整数） | 标志 |
| `+0x1bf8..+0x1c0c` | `0` | 7 个 float |
| `+0x1c10` | `param_3`（长整数指针） | 输出结构 |

E3 `FUN_1800e1740` 是内联 `exp()`：64 项 `2^(k/64)` 三表（`0x1801a4980`、
`0x1801a8820`、`0x1801a8620`）加多项式，不是自定义数学。

E4 常量原值：`0x1801893b8` = `1.0`（double）、`0x18019e040` = `-2432.0`、
`0x1801999e0` = `250.0f`、`0x1801865cc` = `1.0f`。

数值（[推断] 状态机按 `N = rate / [2]` 跳拍，依据是 `+0x20 = frames / [2]`）：

| 采样率 | `[2]` | 攻击系数 | `N` | 时间常数 |
|---|---|---|---|---|
| 48000 | 8 | 1.6203e-3 | 6000 | 102.8 ms |
| 44100 | 8 | 1.7646e-3 | 5512.5 | 102.8 ms |
| 96000 | 16 | 1.6203e-3 | 6000 | 102.8 ms |
| 88200 | 16 | 1.7646e-3 | 5512.5 | 102.8 ms |

四档回到同一时间常数，因为 `2432 / 250 = 9.728` 对应 `1 / 9.728 = 102.8 ms`；
`[2]` 只是把系数折算到归一化采样率。

### 8.3 mixer 是复合状态对象，不是纯路由

E5 `FUN_18003e420`（小音箱）与 `FUN_1800414b0`（LFE 变体）都用
`LocalAlloc(0x40, 0x48)` 加手写 vtable，没有构造函数：

```c
puVar7[1] = CHspIIR2State::vftable;      /* +0x08 */
puVar7[6] = CHspLimiter1State::vftable;  /* +0x30 */
*puVar7   = CBassBoostMixerSmallSpeakers::vftable;   /* 或 CBassBoostMixerLFE */
```

E6 `FUN_18003de90` 把这个 `+0x168` 指针作为第一实参传给 `FUN_180088ff0`，
后者才是通道混合/路由实现（增益表见 §3）。

结论：mixer 持有双二阶与限幅器的**状态**，但它 0x48 字节里没有系数；
滤波系数与增益来自 `FUN_180088ff0` 与全局 DSP 对象。

### 8.4 `CBassBoost` 没有独立内核

E7 `FUN_180089470`（`CBassBoost` 构造）只做两件事：写 `CBassBoost::vftable`，
用 `FUN_1800d8d6c(this+8, 0x28, 0xc, …)` 构造 **12 × 0x28** 数组。没有采样循环。

E8 `FUN_1800894e0`（`CBassManager` 构造）唯一调用 `FUN_180089470(param_1 + 0xc)`，
即 `CBassBoost` 固定在 `CBassManager+0x60`。尺寸自洽：`0x60 + 0x1e8 = 0x248`，
正好是下一段 `memset(+0x248, 0, 0xc0)` 的起点。

E9 `FUN_180089b80`（manager 处理槽）在上游块处理之后二选一：

```c
if ((*(byte*)(this + 0x20) & 8) != 0) FUN_180089780(this - 0x20, block);
else FUN_180088d80(this + 0x40, this + 0x14, block);
```

`this = CBassManager+0x20`，所以 `this+0x40 = CBassManager+0x60 = CBassBoost`，
`this+0x14 = CBassManager+0x34`（声道计数 + 掩码）。

E10 `FUN_180088d80` 逐声道调全局 DSP 对象：

```c
global_dsp->vtable[+0x288](global_dsp, block->channel[i], block->frames,
                           &CBassBoost->slot[i]);   /* 槽 = CBassBoost + 8 + i*0x28 */
```

掩码分支先按 `DAT_180191708[i]`（`i < 12`）过滤，非掩码分支按 `i*5+1` 个 qword
排布，两者落到同一组 12 个槽。

E11 `FUN_180089640` 是 `CBassManager` 析构：恢复两级 vftable，
`_eh_vector_destructor_iterator_(this+0x68, 0x28, 0xc, …)` 析构 12 个槽，
最后把上游源的回指清零。

结论 [已验证]：`CBassBoost` 是 12 个 0x28 槽的状态容器，处理全部委托给
全局 DSP 对象的 `+0x288` 槽；APO 侧没有 Missing Fundamental 谐波核。
`+0x288` 的具体语义（拷贝/延迟/滤波）未解。

### 8.5 两处修正

- **属性描述符跨距**：`FUN_18003e420` 分配 `0xb4` 字节、循环 9 次、每次前进 `0x14`，
  所以每条是 `PROPERTYKEY{GUID(16), pid(4)}`，不是旧记的 `0x18`。
  原始字节在 `0x1801775d0`：`{CEA7DAB2-7E38-4BB6-B32D-ED366A966BF2}`、
  `{D894C4D0-A6B9-4CA6-BF1D-70429B16394D}`、`{90B7F7E6-7DA2-4564-9858-91957A70B552}`，
  各 `pid = 0/1/2`。
- **`+0x1bf0 = 2` 不属于限幅器**：写它的 `FUN_18004d1a0` 是按采样率选系数表的
  采样率转换器（`0x1801f4fc0`/`0x1801f1fb0`/`0x1801f78e0`，`+0x1bf4` = 20/24/36/64 抽头，
  `+0x1bf8 = 0x80`，`+0x1c0c = 0x200`）。同一偏移在两个类里语义不同，不能跨类引用。

### 8.6 `CBassManager` 构造常量

E12 `FUN_1800894e0`：

| 偏移 | 初值 |
|---|---|
| `+0x00` / `+0x20` | `CBassManager::vftable`（主/次） |
| `+0x08` | 上游源指针；同时写 `[source+8] = this` |
| `+0x10..+0x1c` | 源格式 4 个 dword |
| `+0x248` | 清零 `0xc0` 字节 |
| `+0x328` | `-1` |
| `+0x32c` | `0`（bool，属性写 `+0x32c`） |
| `+0x330` | `100` |
| `+0x334` | `1.0f` |
| `+0x338` | `0`（bool） |
| `+0x33c` | `100` |
| `+0x340` | `1.0f` |
| `+0x34` / `+0x3c` | 仅在 `[+0x14] == 5` 时由源格式回填 |

`[+0x14] != 5` 时返回 `0xC00000BB`，与 §3 的 5 声道约束一致。

### 8.7 覆盖矩阵

| 项 | 状态 |
|---|---|
| 限幅器初值与公式 | 已解（E1–E4） |
| mixer 实例化与状态布局 | 已解（E5–E6） |
| `CBassBoost` 是否含内核 | 已解：否（E7–E11） |
| `CBassManager` 构造常量 | 已解（E12） |
| 全局 DSP `vt[+0x288]` 语义 | 未解 |
| `FUN_180088ff0` 如何消费 mixer 内嵌状态 | 未解 |
| LFE 变体（`FUN_1800414b0`）的处理槽 | 未解 |
| `+0x1bec = 250.0` 的读取点 | 未解 |
| 实机听感/扫频回环 | 未执行（本机无 G6 设备） |
