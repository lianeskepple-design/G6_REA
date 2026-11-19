# Gaming SBX 的三项 DSP 静态分析

2026-11-19。样本 KSUSBAPO64.dll，版本 1.2.109.0，SHA-256
f7c649544e8d738df4fb814609f2adc48fbe95091a1ae11ac842910020037156。
以下地址为该样本静态 VA，映像基址为 0x180000000。
证据来自 PE RTTI、虚表、objdump 指令和原始 Gaming 配置。

## E1 · Gaming 默认开关

本机原始配置位于
C:/ProgramData/Creative/SBCommand/Product/G6/Profiles/Gaming.json，
SHA-256 ffb3e85f25a632fa116434499a0d87cbd83de42ba864d5522c3e5f04b8a9558f。
扬声器和耳机两个模式均开启 SBX 总开关、Surround 0.12 和 Crystalizer
0.50。Smart Volume 保存 Level=0.74、Mode=0，但 Enable=false。
Creative.Platform.Devices.dll 中 SoundCoreSmartVolumeFeature 的模式
映射为 Normal=0、Loud=1、Night=2。

## E2 · Surround

CCMSSSurround 对应 RTTI COL 0x1801c3cd0，虚表 0x180196938。
虚表 +0x20 指向 0x180093990，读取 this+0x1c 的有符号整数，除以
100，再由 1+x 与 1−x 计算增益上限为 1 的两支系数，后续按声道掩码分支。
虚表 +0x30 指向处理入口 0x180093940，其中 0x18009394f 调用
0x180093dd0，0x18009397e 调用 0x180093b30，分别对应参数过渡路径与
稳态处理。稳态路径包含声道增益、按掩码的声道复制及双声道组合。
过渡路径 0x180093e59–0x180093eaa 读取当前与目标增益，差值除以
帧数后传给向量内核。0x180093954–0x180093973 在处理后将目标写回
当前增益。这一支使用块内线性过渡。
在对象构建函数 0x18008d180 中，调用点 0x18008d261 构建
CCMSSSurround，另一路按传入选择器构建 CCMSS3UpmixNew
或替代对象，分别保存到主对象的 +8 与 +0。

应用控制层可走两种参数仓库。ApoDeviceRepoKeyFactory 将开关及
强度键映射到 CTPKEY_CMSS3D_Enable 和 CTPKEY_CMSS3D_Immersion。
SoundCoreDeviceConfigCenter 则把同一类应用键映射到
eFeature_Effects_CMSS3D 的参数 0 和 1，经 CTSoundCore 的
SetParamValue 下发。这两条控制路径本身不能确定具体样本处理函数。

同一 DLL 内还有 CCMSS3DEfxMod 与 CStereoSurround3EfxMod 等
Surround 相关实现。上述矩阵处理是已追到的宿主软件路径，Gaming
实际选择的变体、声道矩阵全表及是否运行设备固件分支仍需绑定验证。

### 本轮补充（2026-11-19）

- 参数更新 `0x180093990`：`u = strength/100`，`front = min(1+u, 1)`、
  `extra = min(1-u, 1)`；常量 `0.9535` / `0.3015` 即 `cos/sin(17.5°)`。
  对象当前系数在 `+0x24..+0x34`，目标在 `+0x38..+0x48`。
- 掩码表（`+0x38` 前 / `+0x3c` 额外 / `+0x44` 中置 / `+0x48` LFE）：

  | 掩码 | 前 | 额外 | 中置 | LFE |
  |---|---|---|---|---|
  | 0x03b / 0x033 / 0x603 / 0x60b | 1 | 1−u | 0 | 0 |
  | 0x007 / 0x00f | 0.9535 | 0 | 0.3015 | 0 |
  | 0x037 / 0x03f / 0x607 / 0x60f | 0.2875 | 1−u | 0.3015 | 0.3015 |
  | 0x637 / 0x63b / 0x63f / 0x633 / 0x70f | 0.2875 | (1−u)·0.9535 | 0.3015 | 0.3015 |

- 稳态 `0x180093b30`：前/额外两对按增益复制，中置混入前对、LFE 混入额外对，
  再按掩码把额外对复制到后置输出。
- 过渡 `0x180093dd0`：按块内线性斜坡从当前值到目标值；处理结束后
  `0x180093940` 把目标写回当前并清过渡标志。
- C 参考：[g6_surround.c](g6_surround.c) / [g6_surround.h](g6_surround.h)；
  探针 [surround_test.c](../tools/apo-eq-coeff/surround_test.c) 与
  [surround_ref.py](../tools/apo-eq-coeff/surround_ref.py) 对照通过。
- 未展开：`0x180093daf` 之后的环绕输出复制与少数非标准掩码。

## E3 · Crystalizer

CCrystalizerEfxMod 的主虚表 0x180188ca8 第 8 槽为 0x180042bf0。
该处理入口先调用上游处理器，再按条件处理 128 样本块，核心路径经
0x18008daa0 到 0x18008dec0。后者调用 0x18008e030 分析输入，
调用 0x18008e920 处理左右声道，并按掩码处理第三声道。共享分析以 (L + R) × 0.75
为输入，参见 0x18008e0a1 和标量向量内核 0x18013ca10；随后进入
递归滤波。0x18008e499–0x18008e519 处理 40 槽历史中的 35 条回看。

在分析函数 0x18008e030 中，0x18008e39d 附近可见不对称平滑
（上升新样本权重 0.01，下降 0.001）。0x18008e611 附近生成受
分析状态影响的滤波系数，所以这条路径的响应随输入和状态而变化。
强度处理进入 0x18008ed70，使用四张 18 个 float 的参数表
（0x1801f1e90、0x1801f1ed8、0x1801f1f20、0x1801f1f68）。
旧版内部以 u = strength / 100 为归一值。u ≤ 0.5 时 s = 0.8u，
否则 s = 0.4 + 1.2(u − 0.5)。随后按 s 的 0、0.38、0.75、1 四个
锚点插值。这里的 strength 是旧版内核的输入，未将它等同于
实体 G6 Gaming 的强度下发值。
同一 DLL 的 THX 变体另从 0x18006c430 进入 0x1800b0e70，属于独立路径。
Gaming 的实际变体及完整样本对照仍未验证。

### 旧版 C 移植

代码在 [g6_crystalizer_legacy.c](g6_crystalizer_legacy.c) 与
[g6_crystalizer_legacy.h](g6_crystalizer_legacy.h)，使用 C11。需要显式链接
数学库的平台在链接参数中加 -lm。编译时保留正常浮点顺序，逐位复核使用
MSVC /fp:strict。

- g6_crystalizer_legacy_parameters 接收 0–100 的整数强度，
  以 0x18008ed70 的单精度运算顺序得到 18 个参数。
- g6_crystalizer_legacy_mix 计算上游分析的单声道输入。
- g6_crystalizer_legacy_analyze_block 接受同一段的单声道数据和两条
  上游分析滤波器输出。先取单声道绝对峰值和两路滤波输出各自的绝对值和，
  再按 0x18008e39d–0x18008e60d 更新包络及活动量，其中包含 0.03 下限、
  两次平方根和 40 槽环形历史。写入后从下一写入位置往前进方向扫描
  35 槽，相当于排除最近的 5 条记录。调用前初始化状态，按块保留状态。
- C 代码输出参数和检测状态。完整 PCM 处理还需要上游分析滤波器的配置、
  动态系数到处理滤波器的绑定，以及后续逐声道滤波、混合和输出。

校验使用 MSVC C11 /W4 /WX /fp:strict。参考程序从上述 DLL 直接读出
四组参数表与常量，按反汇编运算顺序用独立 binary32 算术重算。0–100
共 101 档的 1,818 个参数值，以及 160 个输入块的 13,440 个检测状态值
都逐位匹配。覆盖环形缓冲区回绕、短块、空指针、非法强度、非有限输入和
数值溢出拒绝。本轮没有调用 DLL 的音频处理入口或做实体 G6 波形比较。

## E4 · Smart Volume

宿主 DLL 包含 CSVMEfxMod 和 CTHXSVMEfxMod 两套实现。

- 旧版路径的虚表 0x18018c570 第 8 槽指向 0x1800640d0，
  由 0x180064184 以 128 样本块调用 CPlugAGCCompressor 的次
  级虚表。0x180064500 调用初始化 0x180092810，后者将
  0x180196828 写到对象 +0x20；该虚表 +0x08 指向 0x180092ce0，
  其 0x180092d08 调用实际样本循环 0x180092f30。
  Ghidra 已对同一入口生成伪代码，证据 ID
  ev_b3b92335f836a4b396d8730c8446a7517c05676f34e6a293db83678576026862。
- 对旧版样本循环逐条追踪。0x180093025–0x180093087 先做
  一阶状态滤波，再按平方更新每声道的平滑能量；0x18009309d–0x1800930d1
  取参与声道的能量最大值。0x180093221–0x1800932a1 从容量
  2400 样本的环形缓冲区取两个延迟位置，插值后乘平滑增益。
  2400 是缓冲区容量，实际延迟取决于读位置。
- 已确认的局部关系如下，字段偏移相对于压缩器对象。该片段只表达已逐条
  核对的数学关系，强度映射、阈值、时间常数和完整模式行为仍需继续恢复。

~~~
filtered = k1 * (x - previous_x) - k2 * previous_filtered
energy = alpha * filtered^2 + (1 - alpha) * previous_energy
detector = max(energy across selected channels)
output = (mix * delayed_A + (1 - mix) * delayed_B) * smoothed_gain
~~~

- THX 路径的虚表 0x18018dad8 第 8 槽指向 0x180075f60。
  0x180076054 直接调用 0x1800ad330，状态对象按 0x45d8 字节
  步进。参数更新 0x1800767a0 向 0x1800ad7e0 传入索引
  0、2、4、10、11。两条实现的处理函数与状态布局均有区别。

### 本轮补充（2026-11-19）

- 检测器一阶滤波 `0x180092a50` 按采样率选 `k1/k2`；48 kHz 为
  `0.9871 / -0.9742`。逐样本：
  `filtered = (x - prev_x)·k1 - k2·prev_filtered`。
- 能量平滑 `energy = α·filtered² + (1-α)·prev_energy`，
  `α = 1 - 10^(-1/(0.04·fs))`（40 ms）；增益平滑
  `β = 1 - 10^(-1/(0.01·fs))`（10 ms）。
- 攻击/释放：`attack = 1 - 10^(-1/(0.0018638·fs))`（约 1.9 ms），
  `release = 1 - 10^(-1/(8·fs))`（8 s）；斜坡步进 `1/(0.08·fs)`。
- 前瞻：`lookahead = min(0.05·fs, 30)` 样本（48 kHz 为 30）。
- 阈值映射 `0x180093710`：`threshold = (level_db - 12)/10`；默认
  `-6.2` 对应 `level_db = -50`。增益律为
  `gain = 10^(-0.3·max(0, log10(detector) - threshold))`。
- C 参考：[g6_svm.c](g6_svm.c) / [g6_svm.h](g6_svm.h)；
  探针 [svm_test.c](../tools/apo-eq-coeff/svm_test.c) 与
  [svm_ref.py](../tools/apo-eq-coeff/svm_ref.py) 对照通过：静音增益≈1、
  0.5 幅度输入被压到约 0.23、4 秒后释放到约 0.77。
- 未展开：两抽头交叉淡化与各模式（Normal/Loud/Night）的曲线细化。

## E5 · Crystalizer 完整渲染链（2026-11-19 新增）

处理入口 `0x18008daa0` → `0x18008dec0`（128 样本块）→ `0x18008e030`
（分析）→ `0x18008e920`（逐声道）→ `0x18008ec00`（输出级）。

### 分析侧

- 单声道参考 = `(L+R)*0.75`（`0x18008e05b`）。
- 两路 **3 阶 IIR**（4 零点 / 3 极点），系数按采样率类 0..4 取自
  `0x180195A44` / `0x1801959F0`（低通）与 `0x180195AE4` /
  `0x180195A90`（高通）；内核 `0x180147430`：

  ~~~
  w[n] = c8*w1 + c12*w2 + c16*w3 + x[n]
  y[n] = c20*w[n] + c24*w1 + c28*w2 + c32*w3
  ~~~

  48 kHz 实测：低通 200 Hz 以下约 0 dB、1 kHz −25 dB、16 kHz −41 dB；
  高通 20 Hz −191 dB、16 kHz −10 dB、20 kHz −6.9 dB。
- 检测器 `0x18008e030` 的包络、活动量、40 槽历史与既有 C 检测器一致。
  DLL 启动时把两组 40 槽历史全部填为 `2.0f`（`0x18008dcd0`），参考包络
  初值 `0.03f`（`0x18008dcf4`）；既有 C 复位为 0，渲染链初始化改为 2.0。

### 参数

- 强度更新 `0x18008ed70`：`u = strength/100`，`s = 0.8u`（u ≤ 0.5）
  或 `0.4 + 1.2(u−0.5)`；四组 18 参数表按 0 / 0.38 / 0.75 / 1 锚点插值。
- 对象 `+0xC60..+0xCA4` = P0..P17。P0..P4 与 P5..P9 按采样率类选
  S / T；P10 / P11 = 第一 / 第二活动乘数；P12 / P13 = 释放；
  P14 / P15 = 标度 / 阈值。

### 动态系数（`0x18008e611`）

~~~
A = 1 + 4*P11*second_activity
B = 1 + 4*P10*first_activity

一阶：c1 = T
      c2 = 0.5*((A+1) - T*(1-A))
      c3 = 0.5*((1-A) - T*(A+1))

二阶（一阶的平方）：c8 = -2S, c12 = -S^2
      x = 0.5*((B+1) - S*(1-B))
      y = 0.5*(S*(B+1) - (1-B))
      c16 = x^2, c20 = 2xy, c24 = y^2
~~~

三个声道复用同一组系数、状态各自独立；一阶内核 `0x1801468C0`，
二阶内核 `0x1801469E0`。

### 两缓冲重叠相加（`0x18008E920`，2026-11-19 补齐）

逐声道按 128 样本块处理：

1. `scratch[0..127]` ← 上一块 dry（状态 `*(obj+0xC58)+ch*0x200`），
   `scratch[128..255]` ← 当前块；当前块同时写回该槽。
2. 乘 256 点 Hann（`0x180195BD0`）→ 一阶（状态 `+0xE50+ch*0x18`）→
   二阶（`+0xE98+ch*0x28`）→ 再乘 Hann。
3. `+0x1F8` 对应 `FUN_180146190`：`scratch[i] += second[i]*bank[i&3]`，
   权重表 `0x180199E80 = {1,1,1,1}`；`second` 是 `+0x600+ch*0x200`
   保存的上一块 dry，随后被当前块覆盖。因此第二缓冲是纯 dry 叠加，
   不是滤波输出的 OLA。
4. 输出 `scratch[0..127] × ramp`（`0x180195FD0`，`1/(1-0.5·sin²(πn/128))`），
   再经 `+0xF18` 二阶（`0x18008EC00` 的 `+0x2A8`）。
5. 结果有 128 样本延迟：首块输出为零；第二块起叠加 ramp 缩放的上一块 dry。

三个声道为 L / R / center；掩码位 `0x4` 为 0 时不处理第三声道，并把
`+0x400`、`+0xA00` 两处第二缓冲清零（`0x18008DEC0`）。分析侧单声道为
`(L+R)*0.75`（`DAT_18018B7C4`，`0x18008E030` 第 6 参数）。通用 `+0x260`
混合内核 `FUN_180145F30` 为 `(a+b)*0.70710677`（`0x1801B6F50` 三份副本）。

### 代码与验证

- [g6_crystalizer.c](g6_crystalizer.c) / [g6_crystalizer.h](g6_crystalizer.h)：
  完整 128 样本块处理，直接调用 `g6_crystalizer_legacy_analyze_block`；
  `prev_input` / `ola_b` 对应两缓冲，`dry_mix_enabled` 默认开启。
- [crystalizer_chain_test.c](../tools/apo-eq-coeff/crystalizer_chain_test.c)
  与 [crystalizer_chain_ref.py](../tools/apo-eq-coeff/crystalizer_chain_ref.py)：
  两路分析系数、16 点脉冲响应、动态系数与独立 float32 重算逐位一致；
  8192 帧立体声链输出全部有限；0.05 → 0.6 阶跃后第一/第二活动量约为
  0.37 / 0.29，强度 0 与 100 的输出最大差约 5.2e-3，证明检测器已接到
  动态系数与逐声道滤波。
- OLA/混合：首块前 8 帧全零（128 样本延迟）；对同一脉冲输入，开启与
  关闭 `dry_mix_enabled` 的两条链在第二块上的差等于 `上一块 dry × ramp`，
  实测最大误差 `0x00000000`（按位一致）。
- 输出级：`+0xF18` 固定二阶已接入，默认直通，
  `g6_crystalizer_set_output_filter` 可装入实测系数；`0x180195BD0` 的
  256 点 Hann 窗与 `0x180195FD0` 的 128 点增益斜坡已实现。探针校验
  窗口/斜坡值（Hann 与 DLL 表最大差 1.1e-7，斜坡 4.8e-7）及输出级设置。
- 仍未展开：`+0xF18` 系数的初始化位置（`.text` 中未见标量写入，默认
  直通）。强度通过参数表生效。

## 离线客户端

[sbc-dsp-cli.md](sbc-dsp-cli.md) 把 EQ、Crystalizer、Surround、Smart Volume
串成一个 Rust 离线 WAV 客户端：`sbc-dsp --process-wav in.wav out.wav ...`。
C 侧通过 `g6_chain_capi.h` 调用，Rust 只处理 EQ 和文件 I/O。

实时版见 [sbc-live.md](sbc-live.md)：cpal WASAPI 采集/渲染，
麦克风 → EQ → Crystalizer → SVM → 输出，带桌面 GUI 和实时电平。

## 实机待验证

PnPUtil 对当前已连接设备枚举 USB VID_041E/PID_3256，返回无匹配。
本轮的结果是宿主二进制静态证据。G6 的输出模式到底实例化哪个类、哪部分在
设备固件执行、Gaming 的实际效果顺序，仍缺运行时证据。对照信号可按效果
独立设计：Surround 用各输入声道的单独脉冲，Crystalizer 用不同电平的
脉冲串，SVM 手动开启后用分段电平阶跃。记录效果开关、强度和模式，并以
同一输出端点采集对照。原厂波形等价性需由实机数据判定。
