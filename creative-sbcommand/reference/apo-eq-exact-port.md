# G6 APO 十段 EQ 系数：指令级移植与 Rust↔C 对照

2026-11-19。对象 `KSUSBAPO64.dll` 1.2.109.0，SHA-256
`f7c649544e8d738df4fb814609f2adc48fbe95091a1ae11ac842910020037156`，
映像基址 `0x180000000`。本文记录从反汇编到可运行代码的完整链条；所有
地址均为静态 VA，未加载或执行 DLL。

## 1. 三个系数函数与逐样本内核

| 角色 | VA | 证据 |
|---|---|---|
| 峰值段 `CEqFilterPeak` | `0x180087120` | 类 vftable `0x1801964C8` 的 `+0x18` 槽经 `0x1800911D0` 跳入 |
| 低架段 `CEqFilterLowFreq` | `0x180087970` | vftable `0x1801964F0` 的 `+0x18` 经 `0x1800911B0` 跳入 |
| 高架段 `CEqFilterHighFreq` | `0x1800887E0` | vftable `0x180196518` 的 `+0x18` 经 `0x180091190` 跳入 |
| 逐样本内核 | `0x1801469E0` | `0x180090DD0` 经运行时 DSP 表 `[0x180195138+0x2A8]` 调用；另一变体 `0x180147110` |

内核为 4 样本展开的 **Direct Form II**。带对象五个 float32 槽位与两个状态槽：

| 偏移 | 含义 |
|---|---|
| `+0x08` | `c8 = -a1` |
| `+0x0c` | `c12 = -a2` |
| `+0x10` | `c16 = b0` |
| `+0x14` | `c20 = b1` |
| `+0x18` | `c24 = b2` |
| `+0x1c` | `w[n-1]` |
| `+0x20` | `w[n-2]` |

```
w[n] = x[n] + c8*w[n-1] + c12*w[n-2]
y[n] = c16*w[n] + c20*w[n-1] + c24*w[n-2]
```

与标准 `b = [c16,c20,c24]`、`a = [1,-c8,-c12]` 的二阶节等价。`0x180090DD0`
对 8 个中段逐段调用内核；首末段在循环外单独调用。`CEqualiser` 每通道
`0x338` 字节，段步进 `0x50`，共 12 通道。

## 2. 峰值段公式（`0x180087120`）

设 `w = f/fs`、`w0 = 2π·w`、`r = [B+0x40]`（中段为 `f[i+1]/f[i]`）：

```
A = sqrt(gain)
k = tan((w0/r) * 0.5) / A
t = (k - 1) / (k + 1)
c = cos(w0)
c8  = (1 - t) * c
c12 = t
c16 = 0.5*(1+A) - 0.5*(1-A)*t
c20 = -(1 - t) * c
c24 = 0.5*(1-A) - 0.5*(1+A)*t
```

`gain == 0` 时五个槽位清零；`gain == 1` 时写 `c16=1`、其余 0（旁路）。
数值上峰值中心幅度为 `sqrt(gain)`；若 `gain = 10^(dB/10)`，中心即
`10^(dB/20)`，与 UI 的 dB 标签一致。

## 3. 低架段公式（`0x180087970`）

```
A = sqrt(gain)
w0 = 2π·f/fs
x = w0 + (w0/r)*(2/3)        // 指令顺序：除 r、乘 2、除 3、加 w0
s = sin(x), c = cos(x)
den = 1.4 - 0.4*(1/r - 0.707)
S = sqrt((gain+1)/den - gain + 2A - 1)
D = (A-1)*c + (A+1) + S*s
c8  =  2*((A+1)*c + (A-1)) / D
c12 = -((A-1)*c + (A+1) - S*s) / D
c16 = A*(S*s + (A+1) - (A-1)*c) / D
c20 = 2A*((A-1) - (A+1)*c) / D
c24 = A*((A+1) - (A-1)*c - S*s) / D
```

## 4. 高架段公式（`0x1800887E0`）

```
A = sqrt(gain)
w0 = 2π·f/fs
x = w0 - (w0/r)*0.45
s = sin(x), c = cos(x)
den = 1.4 - 2*(1/r - 0.707)
S = sqrt((gain+1)/den - gain + 2A - 1)
D = (A+1) - (A-1)*c + S*s
c8  = -2*((A-1) - (A+1)*c) / D
c12 = -((A+1) - (A-1)*c - S*s) / D
c16 = A*(S*s + (A+1) + (A-1)*c) / D
c20 = -2A*((A+1)*c + (A-1)) / D
c24 = A*((A+1) + (A-1)*c - S*s) / D
```

## 5. 架式段的 ratio

`CEqChan` 构造函数 `0x180090BD0` 把首末段的 `[B+0x40]` 初始化为 `1.0f`；
中段写入 `f[i+1]/f[i]`。全镜像没有对 `[B+0x44]` 的写入，`0x1800916D0`
的 2% 滞后只把 `[B+0x44]` 复制到 `[B+0x40]`，因此架式段实际使用
`r = 1.0`。实测：低架在 31 Hz、高架在 16 kHz、`r = 1` 时 DC/Nyquist
分别落在标签 dB 的 0.01 dB 内。

## 6. 增益约定

DLL 的 `[B+0x28]` 存线性值；属性层把 UI 的 dB 转换到该值的代码尚未定位。
两条公式固定了各自的比例：

- 峰值中心 = `sqrt(gain)`；
- 架式平台 = `gain`。

因此本移植按类型换算：峰值 `gain = 10^(dB/10)`（功率），架式
`gain = 10^(dB/20)`（幅度）。这样两种滤波器都命中 UI 标签。若后续找到
DLL 的真实换算，可直接调用 `*_from_gain` 原始线性入口。

## 7. 代码位置

| 语言 | 文件 | 内容 |
|---|---|---|
| Rust | `app/src/apo_eq.rs` | 三个设计函数、DF2 状态与处理、9 项测试 |
| Rust | `app/src/eq_core.rs` | `Chain::design` 已改用 APO 系数（曲线与 WAV 渲染） |
| C | `reference/g6_eq_apo.c/h` | 同一公式的 float32 实现，独立可编译 |
| C | `reference/g6_eq_dsp.c/h` | 新增 `G6EqApoChain` / DF2 处理接口 |
| 对照 | `app/examples/apo_parity.rs` | 打印 7 组用例的槽位与 256 点脉冲响应 |
| 对照 | `tools/apo-eq-coeff/apo_parity.c` | C 侧同格式输出 |
| 对照 | `tools/apo-eq-coeff/build_parity.cmd` | MSVC `/fp:strict` 编译并运行 |

## 8. 对照结果

7 组用例（峰值 +6/−12 dB、低架 +6/−12 dB、高架 +6/−12 dB，以及 FPS
耳机 10 段链）在 48 kHz 下各输出 5 个系数槽位与 256 点脉冲响应。
Rust 与 C 的十六进制 float32 输出**逐位相同**（含 10 段链），
说明两条实现的运算顺序与常数一致。

复现：

```powershell
cd app
cargo run --example apo_parity > ..\tools\apo-eq-coeff\rust_out.txt
cd ..\tools\apo-eq-coeff
cmd /c build_parity.cmd
# c_out.txt 与 rust_out.txt 除 CRLF 外应完全一致
```

## 9. 仍未确认

1. `[B+0x28]` 的 dB→线性换算位置（属性层未定位）；当前按类型推断。
2. 峰值函数 `gain<1` 分支中 `0x180087368` / `0x1800874A2` 两处未展开。
3. 架式 `S` 项在极低频/极高频的 0/0 极限使频响在 DC/Nyquist 的数值
   评估不稳定；实际处理 31 Hz–16 kHz 频段不受影响，但响应图不应在
   0 Hz/奈奎斯特处取点。
4. 与原厂 DSP、实体 G6 的频响等价性仍未测量。
