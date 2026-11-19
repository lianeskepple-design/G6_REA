# G6 APO 效果结构：虚表簇 / 双二阶系数 / EFXNODE 名称表（静态证据）

目标：`C:\Windows\System32\KSUSBAPO64.dll`（1.2.109.0，PE32+, ImageBase `0x180000000`，符号已剥离）。
方法：纯文件静态解析（pefile + capstone），未运行、未加载、未修改任何系统文件。
本文所有条目都标注 **已观察**（附地址/指令/原始字节）或 **推断**。地址均为 VA（= RVA + `0x180000000`）。

---

## 0. 结论摘要

| 问题 | 结论 | 强度 |
|---|---|---|
| `0x1932A0–0x193388` 是什么 | **不是**扁平函数指针表；是 7 组 MSVC vftable + RTTI COL 槽（30 个 qword）。同簇内含 `CEqFilter` / `CEqFilterPeak` / `CEqFilterLowFreq` / `CEqFilterHighFreq` / `CEqChan` / `CEqChanLFE` / `CEqualiser` | 已观察 |
| 谁在算双二阶系数 | 峰值段 `0x180087120`；低架段 `0x180087970`；高架段 `0x1800887E0`。三者都由各段 vftable 的 **第 4 个虚函数槽（`+0x18`）** 经薄跳板进入 | 已观察 |
| 拓扑 | 10 段 = **1 低架 + 8 峰值 + 1 高架**，每段 **一个二阶节**（`CHspIIR2State`）；不是级联带通 | 已观察（类名 + 构造函数赋值）+ 推断（单节） |
| `a0` 归一化 | 架式两段：**显式**把 5 个系数都除以同一个 a0（连续 5 次 `divss`）。峰段：主路径无 a0 除法，a0 折进 `0.5` 因子 | 架式已观察 / 峰段推断 |
| `EFXNODE_*` | 56 条、连续排布、**每条前带 12 字节头**的结构体数组（内联名字），全镜像 **零指针引用** → 是 MIDL/类型库式元数据，不是运行时派发表；无独立数值 ID | 已观察 |
| Crystalizer / Reverb | 只能从类归属初判（见 §5）；不足以区分谐波生成/梳状全通/卷积 | 推断 |

**与既有笔记的修正**（`reference/apo-eq-implementation.md`）：

1. `0x180196550` 不是"8 项虚表"。它是 `CEqChanLFE` 的 **1 项** vftable；后面 8 个 qword 里夹着 2 个 RTTI COL 指针，属于另外 2 张 vftable（见 §1）。
2. **段步进是 `0x50`，不是 `0x338`**。`0x338` 是"每通道"（`CEqChan`）大小；一个 `CEqChan` = 8 字节头 + 10 × `0x50` 段 + 尾部 3 个字段。
3. "带宽比 `f[i+1]/f[i]` 写在 `+0x14`"：以**段对象基址 B** 计，它写在 **`B+0x40`**；`B+0x28` / `B+0x2C` 才是 1.0f。
4. "拓扑未知"现在可判定（§2）。

---

## 1. 任务一：`0x1932A0–0x193388` 的真实结构

### 1.1 布局（已观察）

`0x1932A0..0x193390` 共 `0xF0` = 240 字节 = **30 个 qword**，紧贴频点表（`0x193390` → VA `0x180196590`）之前。
按 MSVC x64 规则：vftable 的 `[-8]` 槽放 **指向 RTTI CompleteObjectLocator 的完整指针**，其后才是虚函数槽。逐项解出：

| 槽 VA | 内容 | 角色 |
|---|---|---|
| `0x180196498` | `0x1801C3560` | &COL(`.?AVCEqFilter@@`) → 其 vftable 起点 `0x1801964A0` |
| `0x1801964A0..0x1801964B8` | `0x1800909A0`, `0x180090BA0`, `0x1800916D0`, `0x1800D77E8` | `CEqFilter` 的 4 个虚函数 |
| `0x1801964C0` | `0x1801C35E0` | &COL(`.?AVCEqFilterPeak@@`) |
| `0x1801964C8..0x1801964E0` | `0x180090A30`, `0x180090BA0`, `0x1800916D0`, **`0x1800911D0`** | `CEqFilterPeak` 虚表 |
| `0x1801964E8` | `0x1801C3668` | &COL(`.?AVCEqFilterLowFreq@@`) |
| `0x1801964F0..0x180196508` | `0x180090A00`, `0x180090BA0`, `0x1800916D0`, **`0x1800911B0`** | `CEqFilterLowFreq` 虚表 |
| `0x180196510` | `0x1801C36F0` | &COL(`.?AVCEqFilterHighFreq@@`) |
| `0x180196518..0x180196530` | `0x1800909D0`, `0x180090BA0`, `0x1800916D0`, **`0x180091190`** | `CEqFilterHighFreq` 虚表 |
| `0x180196538` | `0x1801C3778` | &COL(`.?AVCEqChan@@`) |
| `0x180196540` | `0x1800908F0` | `CEqChan` 虚表（1 项，删除析构） |
| `0x180196548` | `0x1801C37F0` | &COL(`.?AVCEqChanLFE@@`) |
| `0x180196550` | `0x180090970` | `CEqChanLFE` 虚表（**只有 1 项**） |
| `0x180196558` | `0x1801C3868` | &COL(`.?AVCEqualiser@@`，主基类) |
| `0x180196560..0x180196570` | `0x180090A60`, `0x180091680`, `0x180091380` | `CEqualiser` 主虚表（3 项） |
| `0x180196578` | `0x1801C38F0` | &COL(`.?AVCEqualiser@@`，次基类) |
| `0x180196580..0x180196588` | `0x180091510`, `0x180090B50` | `CEqualiser` 次虚表（2 项） |

类名来源（已观察）：COL 结构 `{signature=1, offset, cdOffset, pTypeDescriptor(RVA), pClassDescriptor(RVA), pSelf(RVA)}`，例如
`0x1801C35E0` 处 = `01 00 00 00 / 00… / pTD=rva 0x… / pSelf=rva 0x1C35E0`，其 TypeDescriptor 名字串为 `.?AVCEqFilterPeak@@`。
全镜像共解出 **7 组** 与本 EQ 相关的 COL：`0x1801C3560/35E0/3668/36F0/3778/37F0/3868/38F0`。

> 顺带纠正：这些函数（`0x180090970/9A0/9D0/A00/A30`）都是"把 vftable 写回 `[rcx]` 后按 `flags&1` 释放对象"的**删除析构**，不是系数函数。`0x180090BA0` 是比较 `B+0x28/+0x2C`、`B+0x40/+0x44`、`B+0x30/+0x34` 三对值、返回"参数是否变化"的叶子函数（未进 `.pdata`，故 `func_of()` 查不到，容易误判为数据）。

### 1.2 段的虚函数语义（已观察）

以 `CEqFilterPeak`（vftable `0x1801964C8`）为例：

| 槽 | 地址 | 语义 | 证据 |
|---|---|---|---|
| `+0x00` | `0x180090A30` | 删除析构 | `mov [rcx],<vtbl>; test dl,1; call operator delete` |
| `+0x08` | `0x180090BA0` | "参数脏了吗" | 3 组 `ucomiss` + `jp/jne`，全相等返回 0 |
| `+0x10` | `0x1800916D0` | 带滞回（2%）的设参 | `mulss 1.02f`(`0x1801965C4`) / `mulss 0.980392158f`(`0x1801965B8`) 夹紧 |
| `+0x18` | `0x1800911D0` | **重算系数薄跳板** | `movss xmm3,[rcx+0x40]; movss xmm2,[rcx+0x30]; movss xmm1,[rcx+0x28]; jmp 0x180087120` |

这直接证实了既有笔记里 "`(**(rax+0x18))()` 提升不进去" 的那一跳就是**系数重算**，且三个参数是 `(gain=[B+0x28], f/fs=[B+0x30], ratio=[B+0x40])`。
三个类的跳板各不同：Peak → `0x180087120`，LowFreq → `0x180087970`，HighFreq → `0x1800887E0`。

### 1.3 段对象布局（已观察，构造函数 `0x180090BD0` / 重算函数交叉验证）

段对象大小 **`0x50`**（`0x180090660` 里对 `[band]` 虚调用后再 `add rbx,0x50`，循环 8 次；`0x1800908F0` 用 `0x1800D6AD4` 以 size=`0x50`、count=8 批量构造）：
基准 `B` = 对象首址（vftable 在此）。

| 偏移 | 内容 | 证据 |
|---|---|---|
| `B+0x00` | vftable | `mov rax,[rcx]; call [rax+0x18]` |
| `B+0x08..0x18` | 5 个系数（重算函数写入） | `movss [rbx+8/0xc/0x10/0x14/0x18]` |
| `B+0x20` | 第 6 个槽（清零路径写 0） | `mov dword [rcx+0x20], eax(0)` |
| `B+0x28` / `B+0x2C` | 当前增益 / 生效增益（线性倍数，初值 1.0f） | `[rcx+0x28]=0x3F800000` |
| `B+0x30` / `B+0x34` | `f/fs`（原始 / 生效） | `divss xmm0,xmm6` + 两次 `movss` |
| `B+0x38` / `B+0x3C` | `f`（Hz）/ `fs` | `movss [rcx+0x38],f`, `[rcx+0x3c]=fs` |
| `B+0x40` | **带宽比 `f[i+1]/f[i]`**（第 1/10 段为 1.0f） | `divss xmm2,xmm1; movss [rbx+0x14]`（`rbx=B+0x2C` ⇒ `B+0x40`） |
| `B+0x44` / `B+0x48` | `fs` / `f` 的副本 | `0x180091250` 中 `movss [rcx+0x44],xmm6`，`0x180090BD0` 中 `[rcx+0x48]=f` |

`0x338` 的对象 = `CEqChan`：vftable + 10 个段（`+0x08 + k*0x50`，k=0..9）+ 尾部 `+0x328`/`+0x32C` = 1.0f、`+0x330` = 0（`0x180090D01`–`0x180090D15`）。

### 1.4 三个系数函数的伪代码要点（任务一要求的"≥3 个目标函数"）

#### (a) 峰值段 `0x180087120`（已观察的关键指令）

```c
void peak_coef(CEqFilterPeak* b, float gain    /*[B+0x28]*/,
                             float w         /*[B+0x30]=f/fs*/,
                             float ratio)    /*[B+0x40]*/
{
  if (gain == 0.0f) { b->c[0..4]=0; return; }              // 0x180087140 ucomiss
  if (gain == 1.0f) { b->B0=1; 其余=0; return; }           // 0x18008717A, 常量 0x1801865CC=1.0f
  w0 = w * 6.28318548f;                                    // 0x180087198, 常量@0x1801916C4 (2π)
  k  = tanf( (w0 / ratio) * 0.5f );                        // 0x1800871B0 divss, 0x1800871B4 mulss 0.5f,
                                                           //   0x1800871BC call 0x1800DADE0
  A  = sqrtf(gain);                                        // 0x1800871C8 call 0x1800DDDD8
  k /= A;                                                  // 0x1800871CD divss
  t  = (k-1)/(k+1);                                        // 0x1800871D9 addss / 0x1800871DE subss / 0x1800871E3 divss
  c  = trig(w0);                                           // 0x1800871E8 call 0x1800DC6C0
  b1 = a1 = -0.5f*(1-A)*c*(1-t);                           // ±0.5f@0x1801916B0, -1.0f@0x180187B58
  b0 = 0.5f*(1+A) - 0.5f*(1-A)*t;                          // → [B+0x10]  0x180087256
  t  → [B+0x0C]                                            //            0x18008727E
  b2 = 0.5f*(1-A) - 0.5f*(1+A)*t;                          // → [B+0x18]  0x1800872A4
  → [B+0x08] = -0.5f*(1-A)*c*(1-t)                         //            0x1800872A9
}
```

要点：
* **出现 2π**（`6.28318548f`，`0x1801916C4`）→ `w0 = 2π·f/fs`，这就是 omega0。
* **除法只有 3 处**：`÷ratio`（`0x1800871B0`）、`÷√gain`（`0x1800871CD`）、`(k-1)/(k+1)`（`0x1800871E3`）。
* **出现"余弦形"项**：`0x1800871E8 call 0x1800DC6C0`（三角函数，见下），其结果只以 `(1-t)` 相乘的形式进入两个相同的系数槽 ⇒ 典型的 `±cos(w0)` 共享项。
* **a0 归一化**：主路径 **没有** 以 a0 为除数的 `divss`；系数是 `±0.5·(1±A)` 形式的加权和 ⇒ 归一化被折进 `0.5` 因子（**推断**）。
* 该函数还有若干分支（`0x180087368`、`0x1800874A2` 处的成对 `divss ... xmm9`）对应 `gain<1`/特殊带宽走法，未逐支展开。

#### (b) 低架段 `0x180087970`（已观察）

* 同样 `w0 = f/fs * 6.28318548f`（`0x180087A0C`），调用 `0x1800DD710`（`0x180087A3A`）与 `0x1800DC6C0`（`0x180087A46`）。
* 额外常量：`2.0f`（`0x180187B54`）、`3.0f`（`0x1801881B8`，`0x180087A2A divss`）、`0.4f`（`0x1801916A4`）、`-2.0f`（`0x1801916D0`）。
* **末尾有一次显式 a0 归一化**：`0x180087B1E/B27/B41/B5D/B61` 连续 5 条 `divss xmm*, xmm0` ⇒ 5 个系数同除 a0（`b2/b1/b0/a1/a2` 全归一）。

#### (c) 高架段 `0x1800887E0`（已观察）

* 同样 2π（`0x18008887B`）、`0.45f`（`0x1801916A8`）、`-2.0f`（`0x1801916D0`），调用 `0x1800DD710` / `0x1800DC6C0` / `0x1800DDDD8`。
* **末尾同样是 5 连除 a0**：`0x18008898B/990/9AB/9CE/9D2`。

> 三个函数都"先算未归一化分子分母、最后统一除 a0"这一模式在架式两段是**指令级可见**的；峰段没有这一步。

### 1.5 三角函数识别（部分推断）

| 地址 | 判据 | 结论 |
|---|---|---|
| `0x1800DADE0` | 小 `|x|` 分支里 `mulsd x³; mulsd c; addsd x`（`0x1800DAE5C`–`0x1800DAE68`，即 `x + c·x³`，c>0），全程双精度，含 3 处 `divss/divsd` | **tanf**（推断：`tan x ≈ x + x³/3`） |
| `0x1800DC6C0` | 以 `|x| ≤ 0x3FE921FB54442D18`（= **π/4**，`0x1800DC708`）分段，`|x|≤π/4` 走 `x²` 多项式，否则做象限变换 | **sinf/cosf 类**（推断；具体是哪一个未定，见 §6） |
| `0x1800DDDD8` | 0xDC 字节、无除法、单次内部调用 `0x1800F1BF0` | **sqrtf/powf(·,0.5)** 类（推断） |

`sinf/cosf/tanf` 均为静态链接（导入表/延迟导入表无槽位，与既有笔记一致），所以只能按常数+结构特征识别。

---

## 2. 任务二：滤波器拓扑判定

### 2.1 结论

**10 段图形 EQ = 第 1 段低架（low shelf）+ 第 2–9 段峰值（peaking）+ 第 10 段高架（high shelf），每段一个二阶节（biquad）。不是搁架×10，也不是级联带通，也无需靠 alpha 反推。**

### 2.2 证据（已观察）

1. **类名**（RTTI TypeDescriptor，§1.1）：`CEqFilterPeak`、`CEqFilterLowFreq`、`CEqFilterHighFreq` 三个并列派生类，而不是"一个类 + 模式枚举"。
2. **每个类有自己的系数函数**：`0x180087120`（峰值）/ `0x180087970`（低架）/ `0x1800887E0`（高架），三者常数集不同（峰值只有 2π/0.5/±1；架式另有 `0.4f`/`0.45f`/`2.0f`/`3.0f`/`-2.0f` 与 5 连除 a0）。
3. **构造函数把不同类钉在首末段**：`CEqChan` 构造函数 `0x180090400`：
   * `0x180090464: lea rax,[rip+0x106085]` → vftable `0x1801964F0` = `CEqFilterLowFreq`，随后写入第 1 段（`[rcx+8]` 附近）；
   * `0x18009048D: call 0x1800D8D6C`（中间 8 段的批量构造/初始化）；
   * `0x1800904F0: lea rax,[rip+0x106021]` → vftable `0x180196518` = `CEqFilterHighFreq`，写入第 10 段（`[rcx+0x2D8]`）。
4. **每段只有一套系数、一个状态类**：段对象 `0x50`，系数槽 `B+0x08..B+0x18`，没有内层节循环；`CEqChan` 析构 `0x1800908F0` 以 size=`0x50`、count=8 批量析构中间段，其元素析构写回的基类 vftable 是 `0x1801886B0`，该 vftable 的 `&COL` 在 `0x1801886A8` = **`.?AVCHspIIR2State@@`**（名字自带 "IIR2" ⇒ 二阶 IIR 状态）。同一 `CHspIIR2State` 也被 BassBoost/Limiter 代码使用（`0x18003DD20/DD60/DDE0` 一带）。
5. **带宽不是 Q 常量**：`divss xmm0, xmm3`（`0x1800871B0`）的除数 `xmm3 = [B+0x40] = f[i+1]/f[i]`，即带宽由相邻频点比决定（指令级坐实既有结论），中间段比值≈2.0 ⇒ `tan(w0/4)`；首末段 `[B+0x40]=1.0f` ⇒ `tan(w0/2)`（架式标准形式）。

### 2.3 未能坐实的一点

* 差分方程形式（DF1 / DF2T）与状态落在哪个槽（`B+0x1C/0x20/0x4C` 三者中）**未确认**——处理循环没有定位到（见 §6）。
* 因此"每段仅 1 个二阶节"来自 `CHspIIR2State` 类名 + 单套系数槽 + 无内层循环三点，标 **推断**。

---

## 3. 任务三：`EFXNODE_*` 名称表的组织方式

### 3.1 组织方式（已观察）

* 共 **56** 条 `EFXNODE_*`，集中连续排布在 `_RDATA` 段：`0x180233C24` … `0x1802343D4`（`_RDATA` VA `0x180226000`，raw `0x217A00`）。
* 每条 = **12 字节头 + 以 `0x00` 结尾的名字**，逐条紧邻（不是指针表，也不是 `{name_ptr, id}`）。示例（raw 见 `0x2247E0` 一带）：

| 名字 VA | 头部 3 个 u32 | 名字 |
|---|---|---|
| `0x180233C24` | `00000ED8  00001438  03B43010` | `EFXNODE_PASSTHRU` |
| `0x180233C40` | `00000ED8  0000189C  09FC300C` | `EFXNODE_SWAP` |
| `0x180233C58` | `00000ED8  00001EEC  EAC73013` | `EFXNODE_CRYSTALIZERW` |
| `0x180233CCC` | `00000ED8  00001AE4  A6AA300F` | `EFXNODE_BASSMGTW` |
| `0x180234104` | `00000ED8  FFFFFFFF  DBA93018` | `EFXNODE_THX_SCOUT_REVERB` |
| `0x1802343C8` | `00000ED8  00002284  641C300D` | `EFXNODE_TOTALWWW` |

* 整段 56 条的**第 1 个 u32 全部等于 `0x0ED8`(=3800)**，是这段的"块类型/块标识"；紧邻其后的另一块（`EffectNodeTypeW`、`IsSupportedW`、`QuerySupportedEffectList`、`ConfigWW`、`NumEffectWWW`、`EffectListW`、`IEffectNodeInfoW`、`__tagAPOHW_INFOW`、`HWTYPEWWh`、`__MIDL___MIDL_itf_CTAPODll_0001_0070_0001` 等）第 1 字段变成 `0x010E`(270) 或 `0x0CE4`。**同一套 12 字节头**，说明它们同属一张表。
* 第 2 个 u32：范围 `0x00000354`–`0x00002284`，**5 条为 `0xFFFFFFFF`**（`THX_SCOUT_REVERB`、`THX_REALITY3D_SCOUT_CMSS3D`、`THX_REALITY3D_CRYSTALIZERWWW`、`THX_STEREO_ENHANCEWW`）→ 更像"偏移/序号"，且允许"无"，**不构成从 0 开始的节点数值 ID**。
* 第 3 个 u32：低字节只有 11/56 条等于名字长度（如 `PASSTHRU`=16⇔`0x10`、`SWAP`=12⇔`0x0C`，但 `CRYSTALIZERW`=20⇔`0x13`、`CMSS1WWW`=16⇔`0x0D`）⇒ 更像哈希/校验，不是长度。

### 3.2 "节点选择是数据驱动"的直接证据

1. **全镜像零引用（已观察）**：对文件内**每一个字节偏移**做 8 字节小端扫描和 4 字节 RVA 扫描，指向这 56 个名字地址（及其 `-4/-8/-12` 头部地址）的指针**数量为 0**；也没有任何 qword 落在整个 `0x180233B00–0x180234600` 区间。⇒ 代码里既没有 `strcmp` 派发，也没有"编译期写死的节点表"。
2. **名字是 MIDL 形状的元数据**：`_W`/`_WW`/`_WWW` 后缀成组出现（`EFXNODE_EQWW`、`EFXNODE_CMSS1WWW`、`EFXNODE_TOTALWWW`），这是 MIDL 对同作用域重名参数的消歧后缀；紧邻的 `__MIDL___MIDL_itf_CTAPODll_0001_0070_0001`、`IEffectNodeInfoW`、`__tagAPOHW_INFOW` 是该 typelib 的接口/结构名。⇒ `EFXNODE_*` 是 `CTAPODll` 类型库里的**参数名常量**，不是运行期分派表（**推断**，但"无引用"是硬证据）。
3. **节点信息通过 COM 暴露，而不是靠内部表**（已观察）：同一块元数据里就是这些方法名 —— `EffectNodeType`(`0x1802343E3`)、`IsSupportedW`(`0x1802343FE`)、`QuerySupportedEffectList`(`0x180234418`)、`ConfigWW`(`0x18023443C`)、`NumEffectWWW`(`0x18023444F`)、`EffectListW`(`0x180234468`)；对应的实现类 RTTI 也在：`CEffectNodeInfo`（COL `0x1801BFFC0`、`0x1801C0160`、`0x1801C0300`）、`CComObject<CEffectNodeInfo>`（`0x1801C0188`、`0x1801C0248`）。
4. **节点类本身由 CLSID/类工厂构造**（已观察其存在）：镜像里有 70+ 个 `CComObject<C*EfxMod>` COL（例如 `CComObject<CCrystalizerEfxMod>` = `0x1801B7F00`/`0x1801B8048`），配套 `CComClassFactory`/`CComObjectCached<CComClassFactory>`（`0x1801BCA38`/`0x1801BCB20`）与 `APOCoClassInterface`（`0x1801C05F0`）。

⇒ 结论：**图（pipeline）由外部（驱动的属性存储 / `IEffectNodeInfo` 查询）按 ID/CLSID 描述，DLL 只按 ID 回答"这个节点是什么/支不支持"并按 CLSID 实例化**；`EFXNODE_*` 只是随类型库链接进来的名字常量。**推断**（依据：无引用 + COM 方法名 + 类工厂）。

### 3.3 未确认

* 12 字节头三个字段的**确切**语义（第 2 字段是否真是偏移、指向哪块数据），以及"外部节点 ID ↔ CLSID"的映射表位置。没有 TLB 资源可对照（`.rsrc` 仅 1 个资源目录，未见 TYPELIB）。

---

## 4. 任务四：`CCrystalizerEfxMod` / `CReverbEfxMod` 初判

类存在性（已观察）：`CCrystalizerEfxMod` COL `0x1801C0D20`/`0x1801C0D48`；`CCrystalizer` `0x1801C33B8`/`0x1801C3440`；`CTHXCrystalizerEfxMod` `0x1801B95B0`（CComObject）；`CReverbEfxMod`（CComObject）`0x1801B81E0`/`0x1801B8328`；另有 `CEarlyRefBuffer` `0x1801C2D98`、`CEarlyRefCircularBuffer` `0x1801C2E10`、`CDelayBuffer<…>` ×4（`0x1801C3BE0`/`0x1801C3FD8`/`0x1801C40E0`/`0x1801C4158`）、`CHspBandCombState` `0x1801C5808`、`CCosineSineTable` `0x1801C5588`、`CHspIIR2State` `0x1801C09C8`。

* **Reverb（推断）**：周边全是延迟线/环缓冲（`CDelayBuffer` ×4、`CEarlyRefBuffer`、`CEarlyRefCircularBuffer`），"early reflection" 命名直接指向 **梳状/全通 + 早反射延迟网络**结构，不太可能是卷积 FFT（本镜像的卷积类会依赖 `CFDAnalysisPlugin`/`CFDSynthesisPlugin`/`CSTFT*`，而 `CReverbEfxMod` 与它们不在同一 COL 区块）。这一条只是结构归属层面的推断，未验证。
* **Crystalizer（推断）**：`CCrystalizer` 与 `CHspIIR2State`（二阶 IIR）同区，`CCosineSineTable` 也在附近，说明其内部是 IIR/查表型运算；**无法**从结构上区分"谐波生成（非线性整形）vs 梳状/全通 vs 卷积"——三者都可能只由一个类 + 内联循环实现。

**阻碍（一句话）**：两个 Mod 的 `Process` 实现没有定位（需要从 `CEfxMod` 基类虚表/`CComObject` 工厂往下追到具体 `Process` 函数），仅凭类名与相邻辅助类只能做归属判断，做不到算法级判定。

---

## 5. 复现方法

```bash
PY="C:/Users/Administrator/AppData/Local/Programs/Python/Python312/python.exe"   # 已装 pefile 2024.8.26 / capstone 6.0.0
# 1) 函数边界：.pdata raw 0x203800，VA 0x180211000，0x12AC8 字节，12 字节/项
#    注意：叶子函数（如 0x180090BA0）不在 .pdata 里，需要按调用点/模式单独反汇编。
# 2) RTTI 名字：扫 .rdata 找 {1,0,0,pTD(RVA),pCHD(RVA),pSelf(RVA)}；名字在 TypeDescriptor+16。
# 3) vftable：vftable[-8] = &COL（完整 8 字节指针）。
# 4) 常数池：0x1801865CC=1.0f, 0x1801916B0=0.5f, 0x1801916C4=2π, 0x180187B58=-1.0f,
#    0x180187B54=2.0f, 0x1801916A4=0.4f, 0x1801916A8=0.45f, 0x1801916D0=-2.0f,
#    0x1801965B8=0.980392158f, 0x1801965C4=1.02f
```

## 6. 未确认清单（明确卡点）

1. **`0x1800DC6C0` 到底是 `sinf` 还是 `cosf`**（已确认是 π/4 分段的三角多项式族）。要定它，需要追 `0x1800DC6C0` 的多项式系数表并看奇偶项（`sin` 只含奇次、`cos` 只含偶次）——本轮未做。
2. **EQ 的处理（差分方程）函数未定位**。按字节模式搜 `mulss/addss` 访问 `disp∈{0x08..0x20}` 的候选里没有命中真正的逐样本循环；说明段系数可能被**内联展开/循环展开**进 `CEqualiser` 的 process，或状态槽不在 `0x08..0x20`。
3. **峰段是否也做 a0 归一化**：主路径未见，但该函数有多个未展开分支（`0x180087368`、`0x1800874A2`），不排除某些分支有。
4. **`EFXNODE_*` 12 字节头字段语义**与"ID↔CLSID"映射位置。
5. **Crystalizer / Reverb 的算法判别**（见 §4 阻碍）。

---

*本文只新增本文件；未修改 `capture/`、未修改任何已有文件、未运行目标进程、未触碰系统文件。*
