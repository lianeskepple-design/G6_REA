# G6 EFX 节点表：56 个 `EFXNODE_*` 的落点、数值 ID 与结构

2026-11-13。目标文件 `C:\Windows\System32\KSUSBAPO64.dll`（1.2.109.0，PE x86-64，已剥离符号，MSVC，
2,347,664 字节，SHA256 `f7c649544e8d738df4fb814609f2adc48fbe95091a1ae11ac842910020037156`——本次自算复核一致）。

> 标注约定：**【观察】** = 字节/接口层面直接读到的；**【推断】** = 由观察推出的解释，可能被推翻。

---

## 0. 结论速览

| 问题 | 结论 | 性质 |
|---|---|---|
| 名字在哪 | 56 条**全部**在 `.rsrc` 里的 **TYPELIB 资源**（type=字符串 `"TYPELIB"`、id=1、lang=1033、RVA `0x22FF50`、29860 字节），**不在 `.rdata`**；全镜像只有这一份（无 ASCII 副本、无 UTF-16 副本） | 观察 |
| 字符串块范围 | 原始偏移 `0x224818` … `0x224FD8`（VA `0x180233C18` … `0x180233FD8`），56 条记录首尾相接 | 观察 |
| 有没有指针表 | **没有**。整个文件里，指向这 56 个名字 VA 的 8 字节绝对指针 = **0 个**，4 字节 RVA = **0 个** | 观察 |
| 数值 ID | **有**：枚举 `__MIDL___MIDL_itf_CTAPODll_0001_0070_0001`（typelib typeinfo[38]，TKIND_ENUM，56 个成员），值 = 声明顺序 **0…55**；`EFXNODE_TOTAL = 55` 是哨兵 ⇒ 真实节点 **0…54 共 55 个** | 观察（由系统类型库读取器读出，非字节猜测） |
| 每条记录的结构 | `[u32 owner][u32 unknown][u8 cch][u8 flags=0x30][u16 hash16][名字 cch 字节][0x57 'W' 填充到 4 字节]` | 观察 |
| `W`/`WW`/`WWW` 后缀 | 是 `0x57 ('W')` 填充字节，不是名字的一部分；`cch` 字节给出的才是真名长度。**填充字节恒为 0x57，可据此在十六进制里切分名字** | 观察 |
| 运行期如何挑节点 | 由调用方按 hardware/endpoint 查询：`IEffectNodeInfo::QuerySupportedEffectList(...)` / `IAPOInfo::QuerySupportedEffectList(...)`，配合 `__tagQUERY_EFFECT_STRUCT{EffectNodeType, IsSupported}`；**DLL 内没有任何静态"节点链"数组** | 观察 + 推断 |

**与旧文档的两处修正**（`reference/apo-effects-structure.md` §3）：

1. 旧结论"无独立数值 ID"不成立：ID 就在 typelib 的枚举里（0…55），只是不在字符串块里、也不在 `.rdata` 里。
2. 旧的"12 字节头 `00000ED8 00001438 03B43010`"已解码：`0x00000ED8` = 拥有该名字的 TypeInfoBase 偏移，`0x00001438` 语义未定，`0x03B43010` = `cch=0x10` / `flags=0x30` / `hash16=0x03B4` 三字段小端叠放。

---

## 1. 节表与换算（本次实测）

| 节 | VA | 原始偏移 | 原始大小 | VA−raw |
|---|---|---|---|---|
| `.text` | `0x180001000` | `0x400` | `0x170200` | `0xc00` |
| `.rdata` | `0x180174000` | `0x170e00` | `0x74800` | `0x3200` |
| `.data` | `0x1801e9000` | `0x1e5600` | `0x1e200` | `0x3a00` |
| `_RDATA` | `0x180226000` | `0x217a00` | `0x8200` | `0xe600` |
| `.rsrc` | `0x18022f000` | `0x21fc00` | `0x8400` | `0xf400` |
| `.pdata` | `0x180211000` | `0x203800` | `0x12c00` | `0xd800` |

本文用到的两条换算（ImageBase = `0x180000000`）：

* `.rsrc`：`VA = 原始偏移 + 0x18000F400`
* `.rdata`：`VA = 原始偏移 + 0x180003200`

> 任务书里给的"`.rdata` VA `0x180174000` → 原始 `0x170e00`"核对一致；但节点名**不在** `.rdata`，见下节。

---

## 2. 资源目录：TYPELIB 资源

| 资源类型 | ID | 语言 | RVA | VA | 原始偏移 | 大小 |
|---|---|---|---|---|---|---|
| 字符串类型 `"REGISTRY"` | 101/110/111/112/113/114/200 | 1033 | `0x22F5D8`… | — | — | 各 213…366 |
| 字符串类型 `"TYPELIB"` | 1 | 1033 | `0x22FF50` | `0x18022FF50` | `0x220B50` | **29860** |
| `16`（RT_VERSION） | 1 | 1033 | `0x22F230` | — | — | 936 |

* 【观察】TYPELIB 数据块首 4 字节是 `4D 53 46 54`（`"MSFT"`），版本字段 `0x00010002`，是标准 MSFT 类型库格式。
* 【观察】块内 `0x74A4` 字节，覆盖原始偏移 `0x220B50`–`0x227FF4`；56 条节点名所在的 `0x224818` 段**在这个区间内部**。
* 【观察】DLL 导入 `LoadTypeLib` / `RegisterTypeLib` / `UnRegisterTypeLib`（来自 `OLEAUT32.dll`）——这是 `DllRegisterServer` 注册内嵌类型库的常规路径，说明这个 tlb 是**给外部客户端看的 COM 元数据**。

⇒ **【推断】** 节点名不是运行期派发用的常量池，而是随类型库编译进来的枚举成员名；DLL 自身代码不需要它们，所以"零引用"是正常的，不是扫描漏了。

---

## 3. 名称记录的真实字节布局

以 `EFXNODE_PASSTHRU` 为例（原始偏移）：

```
0x224818  d8 0e 00 00   ; u32 = 0x00000ED8  owner：所属 TypeInfoBase 偏移
0x22481C  38 14 00 00   ; u32 = 0x00001438  语义未定（见 §9）
0x224820  10 30 b4 03   ; u8 cch=0x10 | u8 flags=0x30 | u16 hash16=0x03B4
0x224824  "EFXNODE_PASSTHRU"   ; 16 字节，恰好 4 对齐，无填充
0x224834  <下一条记录开始>
```

`EFXNODE_CRYSTALIZER`（原始偏移 `0x22484C`）：

```
0x22484C  d8 0e 00 00  ec 1e 00 00  13 30 c7 ea
0x224858  "EFXNODE_CRYSTALIZER"   ; cch = 19
0x22486B  57                     ; 1 字节填充 0x57 ('W')
0x22486C  <下一条记录开始>
```

**【观察】** 56 条记录全部符合：

* 记录长度 = `12 + cch + pad`，`pad = (-cch) & 3`，填充字节**全部** `0x57`（不是 `0x00`）。
* 把整段 374 条 typelib 名字（不止 EFXNODE）都按此规则切分，`pad` 集合只有 `{0, 0x57, 0x5757, 0x575757}`——填充规则是全局的。
* `cch` 恒等于"肉眼看到的字符串去掉尾部 W 之后的长度"（56/56 一致），所以 `EFXNODE_CMSS1WWW` 的真名是 `EFXNODE_CMSS1`。

**【推断】** 这就是任务里"W/WW/WWW 是相邻字符串粘连假象"的机理：**填充字符是 'W'**，所以任何按 C 字符串读法读这一块都会把填充吞进名字里。切名字时用 `cch` 或"去掉尾部 W"都可以，但 `cch` 更可靠。

**【观察·字段含义】**（用同一块里语义已知的名字验证）

| `owner` 值 | 同组名字 | 含义 |
|---|---|---|
| `0x00000000` | `CTAPOGFX`（coclass） | typeinfo[0] |
| `0x00000064` | `IAudioProcessingObject` 及其方法 `Reset`/`GetLatency`/`pTime` | typeinfo[1] |
| `0x000000C8` | `APO_REG_PROPERTIES` 及全部字段名 | typeinfo[2] |
| `0x0000012C` | `APO_FLAG` 及 6 个 `APO_FLAG_*` 成员 | typeinfo[3] |
| **`0x00000ED8`** | **56 个 `EFXNODE_*` 全部** | **typeinfo[38]** |

* 【观察】`owner = 0x64 × typeinfo 索引`（`0x64`=100，tlb 头部 `+0x58` 处就是一张 `0x64, 0xC8, 0x12C, …` 的步长表，TypeInfoBase 在 tlb 内按固定 `0x64` 字节排布）。4 个样本 + 56 条批量一致。
* 【观察】于是 56 条 `EFXNODE_*` 同属**同一个** typeinfo（`0xED8` = 索引 38）。
* 【观察】`flags` 在本块里**恒为 `0x30`**；同一 tlb 内其余名字的 flags 分布为 `0x00`(89) / `0x10`(45) / `0x30`(190) / `0x38`(50)。
* 【推断】`flags` 是"名字种类"：`0x00` = 方法名/参数名（`Reset`、`pTime`、`cbDataSize`），`0x10` = 结构字段名（`clsid`、`szFriendlyName`、`u32MaxInstances`），`0x30` = 常量/枚举成员名（`APO_FLAG_NONE`…、56 个 `EFXNODE_*`），`0x38` = 类型名（`APO_FLAG`、`tWAVEFORMATEX`、`CTAPOGFX`、`__tagQUERY_EFFECT_STRUCT`）。**这个分类正是 §4 能用枚举读回 56 个 ID 的原因**。

---

## 4. 数值 ID：枚举 `__MIDL___MIDL_itf_CTAPODll_0001_0070_0001`

### 方法（可复现、非猜测）

不解析 MSFT 二进制，而是让 Windows 自己解析：

```python
oleaut32!LoadTypeLibEx(r"C:\Windows\System32\KSUSBAPO64.dll", REGKIND_NONE=0, &pITypeLib)
ITypeLib::GetTypeInfo(i)            # i = 0..49，第 50 个返回 0x8002802B (TYPE_E_ELEMENTNOTFOUND)
ITypeLib::GetDocumentation(i)       # 取类型名
ITypeInfo::GetTypeAttr()            # typekind / cFuncs / cVars
ITypeInfo::GetVarDesc(k) + GetNames(memid)   # 取成员名
# 成员值：VARDESC+0x10 的 lpvarValue → VARIANT(vt=3 VT_I4, +0x08 = 值)
```

同一次读取得到 50 个 typeinfo；关键几个：

| idx | kind | 名字 | cFuncs / cVars |
|---|---|---|---|
| 33 | INTERFACE | `IAPOInfo` | 4 / 0 |
| 34 | ALIAS | `APOINFO_HW_IDENTIFIER` | – |
| 35 | **ENUM** | `__MIDL___MIDL_itf_CTAPODll_0001_0070_0002` | 0 / **101**（`APOINFO_HW_*`，0…100） |
| 36 | RECORD | `__tagQUERY_EFFECT_STRUCT` | 0 / 2 |
| 37 | ALIAS | `AUDIO_ARCH_NODE` | – |
| **38** | **ENUM** | **`__MIDL___MIDL_itf_CTAPODll_0001_0070_0001`** | 0 / **56** |
| 39 | INTERFACE | `IEffectNodeInfo` | 1 / 0 |

### 交叉验证（证明读数不是垃圾）

同一方法读 `APO_FLAG`（typeinfo[3]，6 个成员）得到：

```
APO_FLAG_NONE                    = 0
APO_FLAG_INPLACE                 = 1
APO_FLAG_SAMPLESPERFRAME_MUST_MATCH = 2
APO_FLAG_FRAMESPERSECOND_MUST_MATCH = 4
APO_FLAG_BITSPERSAMPLE_MUST_MATCH   = 8
APO_FLAG_DEFAULT                 = 14 (0x0E)
```

位标志语义完全对得上（`DEFAULT` = 2|4|8）。⇒ 读 `EFXNODE` 枚举的值同样可信。

### 结论

* 【观察】`EFXNODE` 枚举 56 个成员，值 **0…55 严格等于声明顺序**（不是位标志）。
* 【观察】最后一个成员 `EFXNODE_TOTAL = 55`，紧接在 `EFXNODE_THX_VOCAL_SUPPRESSOR = 54` 之后。
* 【推断】`TOTAL` 是"节点种类数"哨兵 ⇒ **合法节点 ID 为 0…54，共 55 个**；`55` 只用于数组边界/循环上界。
* 【推断】这套 ID 与 §3 名字块里的 `unknown` 列**无关**（`unknown` 列不是 ID，见 §9）。

---

## 5. 完整 56 条节点表

`名字偏移`/`VA` 指名字首字节；`记录偏移` = 名字偏移 − 12。全部位于 `.rsrc` 的 TYPELIB 资源内。
`pad` = 尾部 `0x57` 填充字节数；`unknown` 见 §9。

| ID | 真名 | 记录偏移 | 名字偏移 | 名字 VA | cch | pad | flags | hash16 | unknown |
|---:|---|---|---|---|---:|---:|---|---|---|
| 0 | `EFXNODE_PASSTHRU` | `0x224818` | `0x224824` | `0x180233C24` | 16 | 0 | `0x30` | `0x03B4` | `0x1438` |
| 1 | `EFXNODE_SWAP` | `0x224834` | `0x224840` | `0x180233C40` | 12 | 0 | `0x30` | `0x09FC` | `0x189C` |
| 2 | `EFXNODE_CRYSTALIZER` | `0x22484C` | `0x224858` | `0x180233C58` | 19 | 1 | `0x30` | `0xEAC7` | `0x1EEC` |
| 3 | `EFXNODE_CMSS1` | `0x22486C` | `0x224878` | `0x180233C78` | 13 | 3 | `0x30` | `0x2560` | `0x1640` |
| 4 | `EFXNODE_CMSS3` | `0x224888` | `0x224894` | `0x180233C94` | 13 | 3 | `0x30` | `0x2562` | `0x1C24` |
| 5 | `EFXNODE_REVERB` | `0x2248A4` | `0x2248B0` | `0x180233CB0` | 14 | 2 | `0x30` | `0x5D17` | `0x155C` |
| 6 | `EFXNODE_BASSMGT` | `0x2248C0` | `0x2248CC` | `0x180233CCC` | 15 | 1 | `0x30` | `0xA6AA` | `0x1AE4` |
| 7 | `EFXNODE_BASSBOOST` | `0x2248DC` | `0x2248E8` | `0x180233CE8` | 17 | 3 | `0x30` | `0x7025` | `0x0418` |
| 8 | `EFXNODE_EQ` | `0x2248FC` | `0x224908` | `0x180233D08` | 10 | 2 | `0x30` | `0x4B14` | `0x1D88` |
| 9 | `EFXNODE_THXSPKCAL` | `0x224914` | `0x224920` | `0x180233D20` | 17 | 3 | `0x30` | `0xD511` | `0x196C` |
| 10 | `EFXNODE_TESTMIX` | `0x224934` | `0x224940` | `0x180233D40` | 15 | 1 | `0x30` | `0x23DA` | `0x1DA8` |
| 11 | `EFXNODE_SVM` | `0x224950` | `0x22495C` | `0x180233D5C` | 11 | 1 | `0x30` | `0x9077` | `0x1944` |
| 12 | `EFXNODE_PITCHSHIFT` | `0x224968` | `0x224974` | `0x180233D74` | 18 | 2 | `0x30` | `0x1648` | `0x1B0C` |
| 13 | `EFXNODE_THX_PASSTHRU` | `0x224988` | `0x224994` | `0x180233D94` | 20 | 0 | `0x30` | `0x1718` | `0x0E08` |
| 14 | `EFXNODE_THX_TESTMIX` | `0x2249A8` | `0x2249B4` | `0x180233DB4` | 19 | 1 | `0x30` | `0xAD36` | `0x1820` |
| 15 | `EFXNODE_THX_MATRIXENCODER` | `0x2249C8` | `0x2249D4` | `0x180233DD4` | 25 | 3 | `0x30` | `0xDB87` | `0x1ECC` |
| 16 | `EFXNODE_THX_CMSS3D` | `0x2249F0` | `0x2249FC` | `0x180233DFC` | 18 | 2 | `0x30` | `0xB45F` | `0x1DE4` |
| 17 | `EFXNODE_THX_SPEAKEREQ` | `0x224A10` | `0x224A1C` | `0x180233E1C` | 21 | 3 | `0x30` | `0xAB87` | `0x2148` |
| 18 | `EFXNODE_THX_CRYSTALIZER` | `0x224A34` | `0x224A40` | `0x180233E40` | 23 | 1 | `0x30` | `0x32D5` | `0x1B28` |
| 19 | `EFXNODE_THX_GRAPHICEQ` | `0x224A58` | `0x224A64` | `0x180233E64` | 21 | 3 | `0x30` | `0xB866` | `0x0354` |
| 20 | `EFXNODE_THX_LIMITER` | `0x224A7C` | `0x224A88` | `0x180233E88` | 19 | 1 | `0x30` | `0x5446` | `0x0F30` |
| 21 | `EFXNODE_THX_BASSMGMT` | `0x224A9C` | `0x224AA8` | `0x180233EA8` | 20 | 0 | `0x30` | `0xBF46` | `0x21FC` |
| 22 | `EFXNODE_THX_BANDSPLIT` | `0x224ABC` | `0x224AC8` | `0x180233EC8` | 21 | 3 | `0x30` | `0x6BD9` | `0x0B70` |
| 23 | `EFXNODE_THX_BANDCOMBINE` | `0x224AE0` | `0x224AEC` | `0x180233EEC` | 23 | 1 | `0x30` | `0xB2B1` | `0x1AA8` |
| 24 | `EFXNODE_THX_CALIBRATOR` | `0x224B04` | `0x224B10` | `0x180233F10` | 22 | 2 | `0x30` | `0x189C` | `0x1CFC` |
| 25 | `EFXNODE_THX_SIMPLEMIX` | `0x224B28` | `0x224B34` | `0x180233F34` | 21 | 3 | `0x30` | `0x710C` | `0x19D8` |
| 26 | `EFXNODE_THX_GAMEEQ` | `0x224B4C` | `0x224B58` | `0x180233F58` | 18 | 2 | `0x30` | `0x4DEB` | `0x12E8` |
| 27 | `EFXNODE_STEREO_SURROUND_FIXED` | `0x224B6C` | `0x224B78` | `0x180233F78` | 29 | 3 | `0x30` | `0x0CE8` | `0x19F8` |
| 28 | `EFXNODE_THX_NOISEREDUCTION` | `0x224B98` | `0x224BA4` | `0x180233FA4` | 26 | 2 | `0x30` | `0x9C79` | `0x1150` |
| 29 | `EFXNODE_THX_MICBEAMPLUS` | `0x224BC0` | `0x224BCC` | `0x180233FCC` | 23 | 1 | `0x30` | `0x0FDD` | `0x0C94` |
| 30 | `EFXNODE_THX_PARAMETRICEQ` | `0x224BE4` | `0x224BF0` | `0x180233FF0` | 24 | 0 | `0x30` | `0x7291` | `0x2094` |
| 31 | `EFXNODE_THX_SVM` | `0x224C08` | `0x224C14` | `0x180234014` | 15 | 1 | `0x30` | `0xE1F3` | `0x09AC` |
| 32 | `EFXNODE_THX_MICBEAM` | `0x224C24` | `0x224C30` | `0x180234030` | 19 | 1 | `0x30` | `0x20AE` | `0x0E9C` |
| 33 | `EFXNODE_ENCODER_DDLIVE` | `0x224C44` | `0x224C50` | `0x180234050` | 22 | 2 | `0x30` | `0x827C` | `0x1FB4` |
| 34 | `EFXNODE_ENCODER_DTS` | `0x224C68` | `0x224C74` | `0x180234074` | 19 | 1 | `0x30` | `0xB54F` | `0x1870` |
| 35 | `EFXNODE_CMSS_DTSNeoPC` | `0x224C88` | `0x224C94` | `0x180234094` | 21 | 3 | `0x30` | `0x361B` | `0x18C0` |
| **36** | **`EFXNODE_THX_SCOUT_CMSS3D`** | `0x224CAC` | `0x224CB8` | `0x1802340B8` | 24 | 0 | `0x30` | `0x0BB7` | `0x1CBC` |
| **37** | **`EFXNODE_THX_SCOUT_GRAPHICEQ`** | `0x224CD0` | `0x224CDC` | `0x1802340DC` | 27 | 1 | `0x30` | `0xC208` | `0x0DF4` |
| **38** | **`EFXNODE_THX_SCOUT_REVERB`** | `0x224CF8` | `0x224D04` | `0x180234104` | 24 | 0 | `0x30` | `0xDBA9` | `0xFFFFFFFF` |
| 39 | `EFXNODE_THX_SPEAKER_CALIBRATOR` | `0x224D1C` | `0x224D28` | `0x180234128` | 30 | 2 | `0x30` | `0xAC05` | `0x0D00` |
| 40 | `EFXNODE_THX_AEC` | `0x224D48` | `0x224D54` | `0x180234154` | 15 | 1 | `0x30` | `0x7F32` | `0x1CA0` |
| 41 | `EFXNODE_AUDIO_POSITION` | `0x224D64` | `0x224D70` | `0x180234170` | 22 | 2 | `0x30` | `0x627E` | `0x0D88` |
| **42** | **`EFXNODE_THX_REALITY3D_SCOUT_CMSS3D`** | `0x224D88` | `0x224D94` | `0x180234194` | 34 | 2 | `0x30` | `0xC680` | `0xFFFFFFFF` |
| **43** | **`EFXNODE_THX_REALITY3D_CRYSTALIZER`** | `0x224DB8` | `0x224DC4` | `0x1802341C4` | 33 | 3 | `0x30` | `0x12BD` | `0xFFFFFFFF` |
| **44** | **`EFXNODE_THX_REALITY3D_BASSMGMT`** | `0x224DE8` | `0x224DF4` | `0x1802341F4` | 30 | 2 | `0x30` | `0x7934` | `0x1F98` |
| **45** | **`EFXNODE_THX_REALITY3D_SCOUT_GRAPHICEQ`** | `0x224E14` | `0x224E20` | `0x180234220` | 37 | 3 | `0x30` | `0x6530` | `0x15FC` |
| **46** | **`EFXNODE_THX_REALITY3D_SCOUT_REVERB`** | `0x224E48` | `0x224E54` | `0x180234254` | 34 | 2 | `0x30` | `0x9633` | `0x14D0` |
| 47 | `EFXNODE_THX_CHANNEL_GRAPHICEQ` | `0x224E78` | `0x224E84` | `0x180234284` | 29 | 3 | `0x30` | `0xCD45` | `0x17F8` |
| 48 | `EFXNODE_THX_KEYCLICK_NS` | `0x224EA4` | `0x224EB0` | `0x1802342B0` | 23 | 1 | `0x30` | `0x2A43` | `0x17CC` |
| 49 | `EFXNODE_THX_MIC_SIGNAL_CONDITION` | `0x224EC8` | `0x224ED4` | `0x1802342D4` | 32 | 0 | `0x30` | `0xBAA6` | `0x0BFC` |
| 50 | `EFXNODE_THX_STEREO_ENHANCE` | `0x224EF4` | `0x224F00` | `0x180234300` | 26 | 2 | `0x30` | `0x506F` | `0xFFFFFFFF` |
| 51 | `EFXNODE_THX_FEEDBACK_SUPPRESSION` | `0x224F1C` | `0x224F28` | `0x180234328` | 32 | 0 | `0x30` | `0xB2A2` | `0x13D8` |
| 52 | `EFXNODE_THX_TDNOISEREDUCTION` | `0x224F48` | `0x224F54` | `0x180234354` | 28 | 0 | `0x30` | `0x56D2` | `0x08B8` |
| 53 | `EFXNODE_VAD_AUTO_MUTE` | `0x224F70` | `0x224F7C` | `0x18023437C` | 21 | 3 | `0x30` | `0x7519` | `0x1A30` |
| 54 | `EFXNODE_THX_VOCAL_SUPPRESSOR` | `0x224F94` | `0x224FA0` | `0x1802343A0` | 28 | 0 | `0x30` | `0x3F5A` | `0x20B4` |
| 55 | `EFXNODE_TOTAL`（哨兵） | `0x224FBC` | `0x224FC8` | `0x1802343C8` | 13 | 3 | `0x30` | `0x641C` | `0x2284` |

（`record / name / cch / pad / flags / hash16 / unknown` 已逐条在 JSON 里给出，含文件里带 W 填充的原始串 `raw_in_file`。）

---

## 6. 负结果：没有指针表（含扫描口径，供复核）

| 扫描 | 范围 | 命中 |
|---|---|---|
| 8 字节对齐 qword == 任一名字 VA（`0x180233C24` … `0x1802343C8`，56 个） | 整个文件 2,347,664 字节 | **0** |
| 4 字节对齐 dword == 任一名字 RVA（`0x233C24` … `0x2343C8`） | 同上 | **0** |
| ASCII `"EFXNODE"` 出现次数 | 同上 | 56（**全部**在 `.rsrc`，无 `.rdata` 副本） |
| UTF-16LE `"EFXNODE"` | 同上 | **0** |
| `.text` 内 RIP 相对引用指向首条名字 VA / TYPELIB 资源基址 | `.text` 全节 | **0** |
| `.data`/`.rdata` 内是否存在 `36,37,38` / `42,45,46` 连续 u32 数组 | 全数据节 | **0**（无静态 Scout 链） |

⇒ 结论：**不存在"名字指针 + 数值 ID"的配对表**。名字与 ID 的唯一连接点是 §4 的 typelib 枚举——
即"第 k 个枚举成员 ↔ 第 k 条名字记录"，两者顺序严格一致（0…55 ↔ `0x224818` 起 56 条记录），
但**没有任何数据结构显式把名字指针和 ID 放在同一行**。

关于任务里提到的 `.rdata` VA `0x180196550`：那里确有 8 字节条目（形如低 32 位 `0x8009xxxx`/`0x801cxxxx`、
高 32 位恒为 `1` 的组合），**但没有一条指向 EFXNODE 名字**（见上表第一行）。紧跟其后的
`0x180196590` 起是 10 个 float32 频点 `31 / 62 / 125 / 250 / 500 / 1k / 2k / 4k / 8k / 16k`，
再接 `0.980392 / 0.999001 / 1.001 / 1.02` 与 `"Manager"` 字符串——与 `reference/g6-scout-geq.md` 记录的频点一致。
该 8 字节条目的语义本任务未解析，且与节点表无关。

---

## 7. 运行期是怎么"数据驱动"选节点的

从同一个 typelib 读出（26 条方法/参数名都对得上，非猜测）：

```
struct __tagQUERY_EFFECT_STRUCT {          // typeinfo[36], 2 字段
    <EFXNODE 值> EffectNodeType;
    <bool/int>   IsSupported;
};

interface IEffectNodeInfo {                // typeinfo[39], oVft 0x18
    HRESULT QuerySupportedEffectList(Hardware, Endpoint, DataFlow, Config, NumEffect, pEffectList);
};

interface IAPOInfo {                       // typeinfo[33]
    HRESULT GetXFiAlgoVersion(...);                       // oVft 0x18
    HRESULT GetAPOFileVersion(...);                       // oVft 0x20
    HRESULT GetAPOArchitecture(pEndpointId, pAPOArchitecture);   // oVft 0x28
    HRESULT QuerySupportedEffectList(pEndpointId, Config, NumEffect, pEffectList); // oVft 0x30
};

typedef AUDIO_ARCH_NODE = <alias>;         // typeinfo[37]
```

* 【推断】调用方（驱动的属性存储 / 上层 APO 宿主）用 `hardware + endpoint + dataflow + config` 去问
  "你支持哪些节点"，DLL 回一个 `EFXNODE`（0…54）数组。**所以"哪台机器用哪几个节点、按什么顺序"不写在
  DLL 里**——这正是"字符串零引用 + 无静态链"的解释。
* 【观察】同一 tlb 里还有一张 101 成员的硬件枚举 `APOINFO_HW_*`（0…100，`APOINFO_HW_UNKNOWN`=0，
  `APOINFO_APOINFO_HW_TOTAL`=100），名字实例包括 `APOINFO_HW_SBX360`(86)、`APOINFO_HW_CINEMA5`(89) 等。
  【推断】`Hardware` 参数就是取这张表的 ID；G6（VID 041E / PID 3256）具体对应哪个 ID 本任务**未取到证据**，
  不做猜测。
* 注意：`EFXNODE`（0…54）与 `APOINFO_HW_*`（0…100）是**两个不同枚举**，别混用。

---

## 8. 分组与相对顺序

【观察】下表就是"枚举声明顺序 = 名字记录顺序"，两者完全一致；分组边界靠命名前缀界定。

| 区段 | ID 区间 | 成员 |
|---|---|---|
| 旧（非 THX）效果块 | 0–12 | `PASSTHRU`, `SWAP`, `CRYSTALIZER`, `CMSS1`, `CMSS3`, `REVERB`, `BASSMGT`, `BASSBOOST`, `EQ`, `THXSPKCAL`, `TESTMIX`, `SVM`, `PITCHSHIFT` |
| THX 主块 | 13–35 | `THX_PASSTHRU` … `THX_MICBEAM`，末尾三个是编码器 `ENCODER_DDLIVE`(33), `ENCODER_DTS`(34), `CMSS_DTSNeoPC`(35) |
| **THX Scout 三元组** | **36–38** | **`THX_SCOUT_CMSS3D`(36) → `THX_SCOUT_GRAPHICEQ`(37) → `THX_SCOUT_REVERB`(38)** |
| 其它 | 39–41 | `THX_SPEAKER_CALIBRATOR`, `THX_AEC`, `AUDIO_POSITION` |
| **Reality3D 块** | **42–46** | `REALITY3D_SCOUT_CMSS3D`(42), `REALITY3D_CRYSTALIZER`(43), `REALITY3D_BASSMGMT`(44), `REALITY3D_SCOUT_GRAPHICEQ`(45), `REALITY3D_SCOUT_REVERB`(46) |
| 麦克风/输入侧 | 47–54 | `THX_CHANNEL_GRAPHICEQ`, `THX_KEYCLICK_NS`, `THX_MIC_SIGNAL_CONDITION`, `THX_STEREO_ENHANCE`, `THX_FEEDBACK_SUPPRESSION`, `THX_TDNOISEREDUCTION`, `VAD_AUTO_MUTE`, `THX_VOCAL_SUPPRESSOR` |
| 哨兵 | 55 | `TOTAL` |

关于 Scout 链路的判断（区分证据强度）：

* 【观察】Scout 相关名字只有 6 个，且分成两组：36/37/38（`THX_SCOUT_*`）与 42/45/46（`THX_REALITY3D_SCOUT_*`）；
  两组的后缀顺序都是 `CMSS3D → GRAPHICEQ → REVERB`。
* 【观察】Reality3D 变体组里 `CRYSTALIZER`(43) 与 `BASSMGMT`(44) **插在** `SCOUT_CMSS3D`(42) 和
  `SCOUT_GRAPHICEQ`(45) 之间，即 **Reality3D 组不是纯 Scout 三元组**；`THX_SCOUT` 组(36–38)才是干净的三个连续。
* 【推断】"Scout = CMSS3D + GraphicEQ + Reverb"（本项目既有结论）在 ID 上对应 **36 → 37 → 38**（THX 机器）
  或 **42 → 45 → 46**（Reality3D 机器）。顺序只是声明顺序，**不等于**信号处理顺序；处理顺序由 §7 的运行期
  查询结果决定，DLL 内无静态证据。
* 【推断】声明顺序看起来是"功能加入的历史顺序"（旧 CMSS 世代 → THX 世代 → Scout → Reality3D 变体 → 麦克风侧），
  不可当作管线拓扑读。

---

## 9. 未决项 / 不要重复的坑

1. **`unknown` 列（记录 +4 的 u32）语义未知**。观察：4 字节对齐、最大 `0x2284`、4 条为 `0xFFFFFFFF`
   （`THX_SCOUT_REVERB`(38)、`REALITY3D_SCOUT_CMSS3D`(42)、`REALITY3D_CRYSTALIZER`(43)、`THX_STEREO_ENHANCE`(50)）。
   它**不是**节点 ID（ID 已由枚举值确定，且它无 0…55 的单调性）。不要拿它当编号用。
2. **ID 空间归属未验证**：tlb 里的 0…54 是 IDL/COM 侧编号；驱动或属性存储侧是否用同一编号，本任务无证据
   （需要抓包或驱动侧符号才能钉死）。
3. **`.data` 的 158 条记录数组（未定论，先记下来免得重查）**：
   原始偏移对应 VA `0x1801EA9B8`，`0x18` 字节一条，共 158 条，字段为
   `{u64 指针 → .data 0x1801E9000 附近的小整数列表, u32 计数(3…14), u32 0, u32 序号 1…158, u32 0}`。
   序号 1…158 **超过** EFXNODE 的 55，说明它不太可能是 EFXNODE 注册表；且全镜像找不到指向该数组的
   8 字节指针，`.text` 内也没找到 RIP 相对 `lea`（按 `target = next_ip + disp32` 全节扫）。⇒ 与节点表的关系**未确认**，
   不要当成"节点参数个数表"直接引用。
4. **`.rdata 0x180196550` 的 8 字节条目**语义未解析；与 EFXNODE 无任何指针关系（已排除）。

---

## 10. 对复刻（Rust 侧）的建议数据模型

```rust
// 与 APO tlb 的 EFXNODE 枚举一一对应；TOTAL 只做边界
#[repr(u32)]
enum EfxNode {
    Passthru = 0, Swap = 1, Crystalizer = 2, Cmss1 = 3, Cmss3 = 4, Reverb = 5,
    BassMgt = 6, BassBoost = 7, Eq = 8, ThxSpkCal = 9, TestMix = 10, Svm = 11, PitchShift = 12,
    ThxPassthru = 13, /* … */ ThxMicBeam = 32, EncoderDdLive = 33, EncoderDts = 34, CmssDtsNeoPc = 35,
    ThxScoutCmss3d = 36, ThxScoutGraphicEq = 37, ThxScoutReverb = 38,
    ThxSpeakerCalibrator = 39, ThxAec = 40, AudioPosition = 41,
    ThxReality3dScoutCmss3d = 42, ThxReality3dCrystalizer = 43, ThxReality3dBassMgt = 44,
    ThxReality3dScoutGraphicEq = 45, ThxReality3dScoutReverb = 46,
    ThxChannelGraphicEq = 47, /* … */ ThxVocalSuppressor = 54,
    Total = 55, // 哨兵
}
```

* 名字→ID 用本文件的表（真名去 W 填充），**不要**用文件里的原始串（带 W）当显示名。
* 节点顺序必须由 profile/配置给（相当于复刻 `QuerySupportedEffectList` 的返回），不要硬编码成 DLL 顺序。
* 若要做 Scout 页，可用 `ThxScoutCmss3d + ThxScoutGraphicEq + ThxScoutReverb`（36/37/38）
  或 Reality3D 变体（42/45/46）择一，取决于目标产品的 `Hardware` 枚举值——G6 对应哪个 `APOINFO_HW_*` 还未取到。

---

## 11. 复现方法（无副作用）

1. **节表/换算**：`pefile.PE(dll, fast_load=True)` 读 section 表（本文件只用只读操作）。
2. **名字扫**：`re.finditer(rb'EFXNODE_[A-Za-z0-9_]*', data)` → 56 命中，全在 `.rsrc`；
   再按 §3 的 `cch`/`0x57` 规则切真名。
3. **指针扫**：对 56 个名字 VA 生成 `struct.pack('<Q', va)` 与 `struct.pack('<I', va-base)`，
   在整个文件上 `bytes.find` 全扫 → 0 命中。
4. **ID 读**：`ctypes` 调 `oleaut32!LoadTypeLibEx(dll, REGKIND_NONE=0, ...)`（用 DLL 自身内嵌 tlb，无需落盘副本），
   然后 `ITypeLib::GetTypeInfo/GetDocumentation` + `ITypeInfo::GetTypeAttr/GetVarDesc/GetNames`；
   `GetVarDesc` 返回的 `VARDESC+0x10` 是 `VARIANT*`，`vt=3 (VT_I4)`，值在 `+0x08`。
   `GetTypeInfo(50)` 会返回 `0x8002802B`，据此确认共 50 个 typeinfo。
5. 全程只读，未修改任何系统文件；未在本仓库落任何中间产物（仅本 `.md` 与同名 `.json`）。

### 文件清单

* `reference/g6-efx-node-table.md`（本文件）
* `reference/g6-efx-node-table.json`（机读版：56 条节点 + 枚举/接口/负结果/未决项）
