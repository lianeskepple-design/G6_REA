# G6 Scout Mode 实现形态（静态证据）

2026-11-12。来源：`C:\Windows\System32\KSUSBAPO64.dll`（1.2.109.0）与 `C:\Program Files (x86)\Creative\Sound Blaster Command\Platform\SndCrUSB.DLL`。机器可读版本见[g6-scout-implementation.json](g6-scout-implementation.json)。

## 结论

Scout Mode 不是一个独立滤波器，而是**三个 EFX 节点的固定组合**，由一个主控参数整体开关：

| 层 | 名称 | 出处 |
|---|---|---|
| 主控参数 | `eParamEfxMasterControl_SCOUTModeMasterOnOff` | SndCrUSB.DLL |
| 节点1 | `EFXNODE_THX_SCOUT_CMSS3D` | KSUSBAPO64.dll |
| 节点2 | `EFXNODE_THX_SCOUT_GRAPHICEQ` | KSUSBAPO64.dll |
| 节点3 | `EFXNODE_THX_SCOUT_REVERB` | KSUSBAPO64.dll |
| RE3D 变体 | `EFXNODE_THX_REALITY3D_SCOUT_CMSS3D` / `_GRAPHICEQ` / `_REVERB` | KSUSBAPO64.dll |

即 Scout ≈ 环绕（CMSS-3D）+ 图形均衡 + 混响 三段的预设组合。这与界面文案“帮助听到接近您和您看到的声音（脚步声、人声、武器打斗声）”一致：靠 EQ 抬中高频，靠 CMSS3D 拉空间感。

## 与 USB 实测的对应

USB 抓到的 `5a 26 05 07 02 00 <00|01> 00` 的作用域字节 02，与 `eParamEfxMasterControl_SCOUTModeMasterOnOff` 对应；同族的 01 对应 `eParamEfxMasterControl_SBXMasterOnOff`。主控参数共 10 个，含 SBX / SCOUT / THX / VIP 四组（各带 `_Features` 与 `_FeaturesLastState` 变体）。

因此“一个字节切换”是合理的：硬件只收到“Scout 总开关 = 1”，具体三段效果如何计算由上层节点配置决定。

## 关键行为：Scout 是互斥预设，不是叠加效果

项目里已有的反编译结果（`call-chain-crosscheck/ApoScoutFeature.cs.txt`）显示：

`ApoScoutFeature.Enable()` 会先保存四个冲突特性的当前状态，然后**把它们逐个关掉**：

| 被保存/关闭 | 记录键 |
|---|---|
| `MasterOnOff`（SBX 主控） | `ScoutMasterOnOffLastStateAttributeId` |
| `GraphicEqualizer`（图形 EQ） | `ScoutEqLastStateAttributeId` |
| `EAXEnvironment`（混响） | `ScoutReverbLastStateAttributeId` |
| Reality3D | `ScoutReality3DLastStateAttributeId` |

`Disable()` 再按记录的 last-state 逐个恢复。因此 Scout 开启期间 SBX 主控、图形 EQ、EAX 混响、Reality3D 都是关闭的——它是抢占式的整体预设。

这与 USB 抓包对得上：关闭 Scout 时设备下发了一整份参数快照（`5a 11 32 08`，id 05 = 0.74 即 SmartVol 74），那就是在把用户原来的 SBX 状态恢复回来，而不是 Scout 本身的数据。

硬件路径是另一个类 `SoundCoreScoutFeature`，它直接调
`Enable(bool surround, bool crystalizer, bool xbass, bool svm, bool dialogplus, bool graphiceq)`
——六个布尔量一次性设定，说明这条路径上 Scout 就是这六个效果的指定组合。

## 10 段频点（APO 内实测数据）

在 KSUSBAPO64.dll 文件偏移 `0x196150` 找到 10 个连续 float32：

```text
31.25  62.5  125  250  500  1000  2000  4000  8000  16000  (Hz)
```

偏移 `0x193390` 另有一组近似表：`31 62 125 250 500 1000 2000 4000 8000 16000`。

`CTPKEY_Scout_GEQ_Gain0..9` 恰好是 10 个频段，因此 Scout 的 GEQ 极可能复用这套频点。这是推断，不是已证事实：频点表属于 APO 的通用 EQ 实现，未直接绑定到 Scout 节点。

## 实现位置

节点名出现在 APO（主机侧音频处理对象）里，而不是 USB 驱动或固件二进制中。

Craft Anything 静态核查（`KSUSBAPO64.dll`，x86_64、已剥离符号、MSVC 构建）显示：`EFXNODE_THX_SCOUT_*` 这些字符串**没有任何函数直接引用**。也就是说节点是按数据（配置表）挑选的，不是硬编码在某段具名例程里。这解释了为什么按字符串找交叉引用会失败。

结论倾向：Scout 的处理由 APO 按节点配置完成，而非 G6 固件内建。但字符串与数据表只能证明"具备这套机制"，不能证明 G6 运行时就跑这三个节点；固件仍可能有等效处理。要坐实必须实测。

## 复刻需要但尚未取得

1. `SCOUT_GRAPHICEQ` 的频点与增益数值（本次未提取到；可能来自预设数据或设备下发）。
2. `SCOUT_CMSS3D` 的参数（环绕强度、延迟）。
3. `SCOUT_REVERB` 的参数（这是之前据 G6 无独立混响页判断“G6 无 Reverb”的反证线索，需复核）。
4. 实测频响，确认三段是否真的作用在音频上。

## 附带产出

同一份扫描得到 56 个 `EFXNODE_*` 节点名与 10 个主控参数名，已写入 JSON，可作为复刻项目的路由词汇表。注意 `W`/`WW` 后缀多为相邻字符串拼接的截断痕迹，不是名称的一部分。
