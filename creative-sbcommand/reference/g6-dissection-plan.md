# G6 完整拆解计划

2026-11-12 起。目标：把 G6 的声音处理链路（APO、驱动、USB 协议、固件边界）尽可能拆干净，为 Rust 复刻提供可验证依据。

## 已完成（有证据）

| 项 | 结论 | 出处 |
|---|---|---|
| 设备识别 | VID041E/PID3256，驱动 1.16.4.26，端点 48k/24bit，播放 8 声道逻辑配置 | 只读枚举 |
| APO 注册 | KSUSBAPO64.dll 1.2.109.0 挂载于播放与输入端点 | 注册表 FxProperties |
| SBX 总开关 | `5a 26 05 07 01 00 <00\|01> 00`，单字节 | USB 抓包 |
| Scout 总开关 | `5a 26 05 07 02 00 <00\|01> 00`，同族不同 scope | USB 抓包 |
| 参数通知通道 | `5a 11 <len> <scope> 00 (96 <id> <float32>)*` | USB 抓包 |
| Scout 语义 | 互斥预设：开启时保存并关闭 SBX 主控/图形EQ/EAX混响/Reality3D | ApoScoutFeature 反编译 |
| Scout 组成 | CMSS3D + GraphicEQ + Reverb 三节点 | APO 节点名 |
| 十段频点 | 31/62/125/250/500/1k/2k/4k/8k/16k，比值≈2（一倍频程） | APO .rdata |
| 带宽 | 非固定 Q：峰值段 `tan(w0/4)`，首末架式段 `tan(w0/2)`（见下方修正） | 指令级 |
| 增益表示 | 线性倍数（1.0=0dB），非 dB | APO 常量 |

## 本轮结果（4 个子代理 + 主线）

| 编号 | 方向 | 状态 | 产物 |
|---|---|---|---|
| A | 参数 ID 映射 | 完成（结论：只确证 2 条） | [g6-parameter-id-map.md](g6-parameter-id-map.md) / .json |
| B | EFX 节点表 | 完成（56 节点 ID 0–54 已解） | [g6-efx-node-table.md](g6-efx-node-table.md) / .json |
| C | Scout GEQ 默认增益 | 完成（**未找到**，附完整排除范围） | [g6-scout-geq.md](g6-scout-geq.md) |
| D | 混响 / Crystalizer 结构 | 完成（EQ 拓扑被彻底解出） | [apo-effects-structure.md](apo-effects-structure.md) |
| F | 滤波器拓扑与 Q | **已解**（原判受阻系误判） | [apo-effects-structure.md](apo-effects-structure.md) |
| E | EQ 单频段 USB 抓包 | 待做（2026-11-13：G6 未连接，USBPcap 服务与 SBCommand 均在运行但无设备，无法抓包） | — |

### 重大修正

1. **EQ 拓扑已定**：10 段 = 1 低架 + 8 峰值 + 1 高架，每段一个二阶节。原先"虚调用无法跟进"的判断是错的——走的是各段 vftable 第 4 槽（`+0x18`）薄跳板，目标函数分别是 `0x180087120`（峰值）/ `0x180087970`（低架）/ `0x1800887E0`（高架）。
2. **Q 结论作废**：我先前提的"全段 Q≈1.4142"错误。实际峰值段 `alpha = tan(w0/4)`（等效 Q≈2.0，8 kHz 处降到 1.62），首末架式段 `alpha = tan(w0/2)`。不是固定 Q，也不能用一个常数替换。
3. **段步进是 `0x50` 不是 `0x338`**：`0x338` 是每通道（`CEqChan`）的大小，一个通道 = 8 字节头 + 10×`0x50` 段。
4. **USB 通知第 4 字节是条目数不是 scope**：`len = 2 + 6×count`，四个样本无一例外。
5. **id 00 = SBX 主控**，不是 Scout 启用位；Scout 开启时它变 0 是"强制关 SBX"的正常表现。
6. **EFX 节点有 ID**：0–54（`EFXNODE_TOTAL=55` 为哨兵），来自内嵌 TYPELIB 枚举声明顺序；`W` 后缀是 0x57 填充字节，不是名字的一部分。

## 已知阻碍

- `(**(rax+0x18))()` 为虚调用，目标运行时决定；`sinf/cosf/tanf` 为静态链接（无 IAT 槽位），无法按调用点追。
- `cfSB1770.ini` 为二进制且高熵（6.96），APO 内无 `.ini` 字符串与密码学导入，读取方尚未定位。
- Scout GEQ 的 10 个增益值不在注册表、APO INI、托管代码中。

## 纪律

- 只读原厂文件；不改 `app/` 源码；不创建本清单以外的文件。
- 结论必须标注证据出处与置信度；取不到就写 null 结果，不猜。
- 该目录不是 Git 仓库，不做提交。
