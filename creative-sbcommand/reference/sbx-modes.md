# SBX / Scout / Direct 模式语义

2026-11-21。数据与依据：`reference/g6-product-data.json`（23 个工厂 profile）、
`reference/g6-scout-geq.md`、`reference/g6-profiles.md`。实现：`app/src/sbx_modes.rs`，
由 `sbc-live`（实时）与 `sbc-dsp`（离线）共用。

## 1. 为什么需要这一层

在这之前 `sbc-live` 把 Crystalizer / Bass / Dialog+ / Smart Volume / EQ 当成五个互不相干的开关，
这与产品语义不符：

- G6 的效果开关是 **SBX 主控 + 整套 profile**，不是自由组合；
- Scout 与 SBX 互斥：开 Scout 会保存并关闭 SBX 主控、GraphicEQ、EAX、Reality3D；
- 长按 Scout 进入 Direct，即全部旁路。

## 2. profile 里到底有什么

每个 profile 存的是**每个效果的 Enable + Level(0..1)**，不是只有五个电平：

```json
"Gaming": {
  "Surround":   {"Enable": true,  "Level": 0.12},
  "Crystalizer":{"Enable": true,  "Level": 0.50},
  "Bass":       {"Enable": false, "Level": 0.50, "XOver": 80},
  "SVM":        {"Enable": false, "Level": 0.74, "Mode": 0},
  "DialogPlus": {"Enable": false, "Level": 0.50},
  "Scout":      {"Enable": false},
  "SBXMaster":  {"Enable": true}
}
```

统计（23 个 profile）：

| 观察 | 结果 |
|---|---|
| `SBXMaster.Enable` | 23/23 为 on——互斥是运行时开关，不是 profile 数据 |
| `Scout.Enable` | 23/23 为 off——同上 |
| `Bass.XOver` | 23/23 为 80 Hz |
| `SVM.Mode` | 23/23 为 0，与 `CPlugAGCCompressor` 没有模式曲线一致 |
| Speaker 与 Headphone | 只有 2 个 profile 有差异，且都不在五个效果字段上 |
| Enable 实际差异 | Gaming 关掉 Bass/SVM/Dialog+；Movie 打开 SVM(0.40)；RPG 打开 Bass(0.20)+Dialog+(0.15) |

因此"套用 profile"必须同时写 Enable 与 Level，只写电平会得到错误状态。

## 3. 三种模式

| 模式 | 语义 | 主机侧行为 |
|---|---|---|
| `Direct` | 长按 Scout | 全部效果旁路，含 EQ；`sbc-dsp` 输出与输入逐位一致 |
| `Sbx` | SBX 主控 on + 一个 profile | 按 profile 的 Enable/Level 配置各效果 |
| `Scout` | Scout on，SBX off | 关闭 SBX、EQ 与全部效果；**Scout 的 DSP 不在主机侧** |

`EffectLevels::bypass_all()` = `direct || scout || !sbx_master`，`sbc-live` 用它决定是否禁用
效果控件，`sbc-dsp` 用它决定是否加载任何一级处理。

Scout 这一项目前只有互斥语义：`KSUSBAPO64.dll` 里没有 Scout 算法（只有 USB 开关与
`CTPKEY_Scout_*` 参数通路），所以客户端不会假装它在处理音频，UI 与 CLI 都会明确说明。

## 4. 两处刻意的近似

1. **SVM 电平到 dB**：产品存 0..1，二进制里没有任何电平旋钮（`+0x4b98` 构造外无写入点）。
   `EffectLevels::smart_volume_db()` 用 `-50 + 0.3 * level` 覆盖客户端已有的 -50..0 dB 滑杆，
   这是**有标注的近似**，不是恢复出来的常量。
2. **Surround mask**：profile 不带 mask，`sbc-dsp` 在 6 声道输入上固定用 `0x3f`（5.1）；
   非 6 声道会跳过并警告。`sbc-live` 目前是立体声链，profile 的 Surround 只记录不生效。

## 5. 用法

实时客户端：`sbc-live` 顶部 Mode 行切换 Direct / SBX / Scout；选 SBX 时出现 profile 下拉，
选中即加载（随后滑杆仍可微调，但产品语义上已不是该 profile）。Direct / Scout 下效果控件禁用。

离线：

```powershell
# 套用工厂 profile（隐含 --mode sbx）
sbc-dsp --process-wav in.wav out.wav --sbx-profile Gaming
# 全旁路，输出与输入逐位一致
sbc-dsp --process-wav in.wav out.wav --mode direct
# Scout：与 Direct 相同的旁路，并提示 Scout DSP 不在主机侧
sbc-dsp --process-wav in.wav out.wav --mode scout
```

冲突会被拒绝：`--mode direct/scout` 与任何效果参数或 `--eq-preset` 同时使用；
`--mode sbx` 缺 `--sbx-profile`；未知 profile/mode。

## 6. 验证

| 检查 | 结果 |
|---|---|
| `cargo test --lib` | 29 passed（新增 7 个：23 条 profile、Gaming/Movie/RPG/Dota2 的 Enable、大小写查找、Direct/Scout 旁路、dB 近似） |
| `sbc-dsp --mode direct` vs 输入 | **逐字节一致**（1048576 字节） |
| `sbc-dsp --sbx-profile Gaming` vs 输入 | 有差异（Crystalizer 50%） |
| `sbc-dsp --sbx-profile Movie` vs Gaming | 有差异（Movie 打开 SVM 0.40） |
| 6 声道 + `--sbx-profile Gaming` | 走 Surround 分支，无跳过警告 |
| 错误路径 | 5 种冲突/未知参数全部按预期报错且不产出文件 |
| `sbc-live` GUI | 启动 6 秒无 panic（rc=124），`--selftest 1` 正常 |

## 7. 未完成

- profile 与 EQ 预设没有映射关系（产品里 EQ 页独立），所以套用 profile 不改 EQ。
- `sbc-live` 的 EQ 只在 Start 时生效，运行中改模式不会重设 EQ。
- Scout 的实机行为（子带自适应增益）仍需要 G6 与抓包。
