# 无声卡离线工作进展

日期：2026-11-12（UTC+8）。本轮已完成参数静态分析、原厂数据审计及可用的离线EQ文件处理。没有连接/打开音频设备、录音、播放或抓包；未加载执行厂商DLL，未改注册表。

## 1. 可用的离线处理器

- 程序：`../app/target/debug/sbc-eq.exe`；源码：`../app/src/bin/sbc-eq.rs`。
- 编译内嵌34个G6原厂EQ预设，分别保留Speaker、Headphone和PreAmp。
- 10段滤波按 APO 实测拓扑：第1段低架、第2–9段峰值（alpha=tan(w0/(2·ratio))）、第10段高架。峰值段带宽来自相邻频点比，不再是固定 Q=1.0。架式段用 RBJ 斜率 S=1 近似（APO 架式内部公式未完全还原）。
- PCM16/24/32与float32 WAV输入，float32输出；保留采样率、声道、帧数。
- 不覆盖原始文件或已有输出，错误与非有限输出以非零退出，拒绝输出Inf半成品。
- `--headroom-db`为带符号附加增益，负数衰减、正数增加；不是自动限幅。
- 增加Cargo默认运行目标，`cargo run`继续启动原桌面界面；本轮文件处理使用独立CLI，尚未接入桌面按钮或实时声音链。

在项目根目录PowerShell示例（替换实际输入路径）：

```powershell
& './app/target/debug/sbc-eq.exe' --list-eq-presets
& './app/target/debug/sbc-eq.exe' --process-wav './input.wav' './output_EQ.wav' --eq-preset FootstepEnhancer --output-mode headphones --headroom-db -6
```

## 2. 最终验证

- 完整cargo build通过（桌面程序与离线处理器）。
- cargo test：当轮离线EQ 6 项通过；2026-11-13 起工作区共 19 项（见第 6 节）。
- 实际文件验收23/23通过（当轮记录），记录在offline-independent-results.json；当轮二进制SHA256为`dce6fd5d…`，已被 2026-11-13 重建取代，当前值见第 6 节。
- FootstepEnhancer脉冲导出与SciPy SOS参考最大绝对差：`2.2665778143249327e-9`。这是所选软件滤波器的数值对照，不是与G6硬件的对照。
- 覆盖Flat直通、正负dB、声道隔离、4种WAV输入、双模式差异、重复运行一致、同路径/硬链接/已有输出保护及NaN/Inf/极大增益拒绝。
- 复现脚本：`../tools/verify-offline-eq.py`、`../tools/verify-offline-eq-independent.py`；详细记录：`../tools/offline-eq-results.md`。

## 3. 参数与调用链

- 参数报告：parameter-ranges.md/json；证据索引：offline-contracts/README.md。
- GraphicEQ平台封装：单段-24至+24 dB，Granularity=1、默认0；Preamp -12至+12 dB、Granularity=1、默认0。
- XBass频率10至1000 Hz、默认80；强度按仓储/产品分为0至1或0至100；G6不装配此封装的扬声器BassManagement分支。
- Granularity不是SetValue硬性量化规则；硬件真正允许的范围仍须设备GetParamInfo确认。
- 复现确认Devices有56处范围构造位置；UI Framework另有1处动态构造，不应混作静态常量。
- 主对话独立交叉证据：call-chain-crosscheck/README.md。

## 4. 数据审计与旧报告修正

- G6：23组SBX profile、34组EQ，导出设置与原厂逐项一致。
- G6有18组EQ的Speaker/Headphone不同。
- 与AE-9同名的29组EQ中，Speaker有6组、Headphone有7组不同；撤回旧“全部相同”结论。
- FPS、CallofDutySeries、DrivingSimulation仅Speaker全零，耳机曲线非零。
- 混响608条=437 Float、152 Bool、19 Integer；每组参数1至31加ActivePreset=258，不是608个浮点数。
- 审计与源文件哈希：factory-data-audit.md/json；完整双模式表：g6-eq-presets.md。
- 撤回仅凭字符串未命中断言“算法不存在”“G6无混响”等结论。

## 5. 尚未完成及边界

原厂滤波器Q/拓扑、硬件频响、Scout曲线、运行时参数范围和APO/固件实际分工仍未知。SBX其他效果、XBass、混响及实时音频输入输出尚未实现。现有离线EQ不能称为原厂DSP的等价复刻。

反编译中间树、安装过程临时内容及验收音频已清理；保留源码、精简证据、复现脚本、依赖及正常构建产物。

## 6. 2026-11-13：桌面端接入与文档回填

- 滤波内核抽到 `../app/src/eq_core.rs`（库目标 `aurora_audio_command`），CLI 与桌面共用同一份实现；`app/src/bin/sbc-eq.rs` 只剩参数解析。
- 桌面 Equalizer 页不再使用手写的 15 条 AE5Plus 曲线：预置列表改为编译内嵌的 34 条 G6 原厂数据（`reference/g6-eq-presets.json`），按输出模式（Speaker/Headphone）取对应曲线，18 组双模式差异原样保留。
- 曲线图改为用同一内核逐点计算幅频响应（31 Hz–16 kHz 对数轴、含 preamp），不再是频段点之间的样条插值；图上叠加“Computed peak … dB @ …”。
- Equalizer 页新增 “Render WAV…” 对话框：把当前 10 段增益 + preamp 直接渲染成 float32 WAV，输出路径默认 `<输入名>_eq.wav`，已存在文件拒绝覆盖；错误信息显示在对话框内。
- 上一版记录中的两处数据已修正：单元测试现为 19 项（eq_core 13 + 桌面 6），二进制哈希取本轮重建值。

本轮验证（当前树）：

| 检查 | 结果 |
|---|---|
| `cargo test` | 19 passed（eq_core 13、桌面 6） |
| `verify-offline-eq.py` | E2E PASS，覆盖项与上表一致 |
| `verify-offline-eq-independent.py` | 23/23 |
| `sbc-eq --list-eq-presets` | 34 行 |
| 桌面截图（`--screenshot`，AURORA_PAGE=equalizer） | Flat 与 Fortnite 两次渲染正常，曲线形状与频段增益一致 |
| 当前二进制 SHA256 | sbc-eq `7708efe035c16985c34b11ce12d1ede47373adb0f37878a72c11afaa080bd8d4`；桌面 `b83ec768a8a032f64f16be1b5dea0ef68751e400e9241209be00e7ff37af314a` |

界面与实时链的边界不变：桌面仍不做实时音频输入/输出，只做曲线显示与离线 WAV 渲染；原厂 DSP 等价性仍未被证明。

## 7. 2026-11-19：APO 系数指令级移植

- 反汇编并解码三个系数函数：峰值 `0x180087120`、低架 `0x180087970`、高架 `0x1800887E0`；逐样本内核 `0x1801469E0` 为 Direct Form II，系数槽位 `+0x08..+0x18`，状态 `+0x1c/+0x20`。
- 新增 `app/src/apo_eq.rs`（9 项测试），`eq_core::Chain::design` 切到 APO 系数；桌面曲线、`sbc-eq` 与 WAV 渲染随之一致。
- 新增 `reference/g6_eq_apo.c/h` 与 `G6EqApoChain`（float32 / DF2）；`verify-g6-eq-presets.ps1` 增加 APO 旁路、非平直、声道隔离与峰值槽位参考检查。
- Rust↔C 对照：7 组用例（峰值/低架/高架各 ±、FPS 10 段链），每组 5 个系数槽位 + 256 点脉冲响应，逐位相同。
- 关键修正：架式段 `ratio = 1.0`（旧文档的 2 不成立）；低架频率扭曲项为 `2/3`（不是 `1/6`）；峰值按功率比、架式按幅度比换算标签 dB。
- 仍未确认：属性层 dB→线性换算位置、峰值 `gain<1` 分支、DC/Nyquist 的 0/0 数值评估、实体 G6 频响。

本轮验证（当前树）：

| 检查 | 结果 |
|---|---|
| `cargo test` | 28 passed（eq_core 13、apo_eq 9、桌面 6） |
| `verify-g6-eq-presets.ps1` | PASS，含 APO 精确链检查 |
| Rust↔C 位级对照 | 7/7 用例逐位一致 |
| 原厂系数证据 | `reference/apo-eq-exact-port.md` |

## 8. 2026-11-19：Crystalizer 渲染链

- 解出对象初始化 `0x18008DAF0`：5 类采样率（22050/44100/48000/96000/192000）、两路 3 阶分析滤波器系数表、40 槽历史初值 `2.0f`、参考包络 `0.03f`。
- 解码 `0x180147430` 的 3 阶 IIR 内核与 `0x1801468C0` 的一阶内核；`0x1801469E0` 的二阶内核与 EQ 相同。
- 解码 `0x18008E611..0x18008E737` 的动态系数：一阶 `c1/c2/c3` 与二阶 `(x + y·z⁻¹)² / (1 + S·z⁻¹)²`，由两路活动量经 P10/P11 调制。
- 新增 `reference/g6_crystalizer.c/h`：128 样本块处理，单声道 `(L+R)*0.75`，两路 3 阶分析滤波，直接调用已逐位验证的 `g6_crystalizer_legacy_analyze_block`，再按动态系数做逐声道一阶 + 二阶滤波。
- 输出级：`+0xF18` 固定二阶已接入（**2026-11-21 更正：该级不存在，见 §16**）；`0x180195BD0` 的 256 点 Hann 窗与 `0x180195FD0` 的 128 点增益斜坡 `1/(1-0.5·sin²(πn/128))` 已实现，探针校验窗口/斜坡值。
- 对照：两路分析系数、16 点脉冲响应、动态系数与独立 float32 重算逐位一致；8192 帧立体声链输出全部有限；0.05 → 0.6 阶跃后第一/第二活动量约 0.37 / 0.29，强度 0 与 100 的输出最大差约 5.2e-3，证明检测器已驱动动态系数与逐声道滤波。
- 两缓冲：`0x18008E920` 每声道取 `[上一块 dry, 当前块]` 共 256 点，乘 Hann → 一阶 → 二阶 → 再乘 Hann；`+0x1F8`（`FUN_180146190`）把 `+0x600` 第二缓冲的上一块 dry 以权重表 `0x180199E80 = {1,1,1,1}` 加到 scratch，随后两缓冲都写入当前 dry；输出取前 128 点乘 ramp，再经 `+0xF18`。首块为零，存在 128 样本延迟。
- 仍未展开：`+0xF18` 系数初始化位置（默认直通，可设为实测值）。**2026-11-21 撤回：位移 `0xf18` 在 `.text` 中 0 次命中，不存在这一级。**

| 检查 | 结果 |
|---|---|
| `crystalizer_chain_test.exe` | 编译 `/W4 /WX /fp:strict` 通过 |
| `crystalizer_chain_ref.py` | PASS（分析系数/脉冲/动态系数逐位一致，128 样本延迟，dry 叠加按位一致，块输出有限） |
| `cargo test` | 28 passed（本轮未改动 Rust） |

## 9. 2026-11-19：Surround 与 Smart Volume

- Surround：`0x180093990` 把掩码与强度映射到五个目标增益（前/额外/中置/LFE），`0x180093B30` 稳态、`0x180093DD0` 过渡；常量 0.9535/0.3015 为 17.5° 的 cos/sin。
- 掩码表覆盖 0x03b/0x033/0x603/0x60b、0x007/0x00f、0x037/0x03f/0x607/0x60f、0x637/0x63b/0x63f/0x633/0x70f；稳态路径做前/额外增益、中置→前、LFE→额外及后置复制；过渡路径按块内线性斜坡。
- 证据分级：函数地址、常量、13 个 mask 分支和稳态/过渡公式是反编译并用 `surround_ref.py` 逐位对拍；mask 的 FL/FR/FC/LFE/BL/BR 声道语义是按通道顺序和产品资料推断，未做 G6 实机逐通道验证；没有 HRTF FIR/ITD/IID。
- SVM：`0x180092A50` 给出按采样率的 k1/k2（48 kHz 0.9871/-0.9742）；`0x180092DA0` 给出 40 ms 能量平滑、10 ms 增益平滑、1.9 ms 攻击、8 s 释放、80 ms 斜坡与 30 样本前瞻；`0x180092F30` 为样本循环。
- 新增 `reference/g6_surround.c/h` 与 `reference/g6_svm.c/h`；探针 `surround_test.c` / `svm_test.c` 与 `surround_ref.py` / `svm_ref.py` 对照通过。

| 检查 | 结果 |
|---|---|
| `surround_ref.py` | PASS（13 个掩码表、稳态矩阵、8 样本过渡斜坡） |
| `svm_ref.py` | PASS（k1/k2/时间常数、静音增益 1、0.5 输入压到 0.23、4 s 释放到 0.77） |
| 编译 | MSVC C11 `/W4 /WX /fp:strict` 通过 |

## 10. 2026-11-19：Scout 取数工具与 Crystalizer 输出级

- 新增 `tools/scout-measure/`：`generate-sweep.py`（Farina 扫频 + 反卷积参考）、`analyze-response.py`（FFT 反卷积、10 段 ΔdB、preamp 估计、合成 APO 链自测）、`measure-scout.py`（自测/分析/录音编排）、`usbpcap-single-band.py`（USBPcap 抓包 + `5a 11` 通知解析 + 单频段差分）、`README.md`。
- 新增 `run-g6-hardware.ps1` / `hardware-runbook.md`：预检 PnP、音频端点与 USBPcap 服务 → Scout 扫频 → 31 Hz +6 dB 自检 → 逐频段 USBPcap 映射 → `g6-hardware-report.json`。`-WaitSeconds N` 可轮询等待插入/上电；本机无 G6，预检按设计返回退出码 2（`Get-CimInstance` 无 `VID_041E&PID_3256`，`sounddevice` 无 G6 端点；注册表仅有历史驱动键）。
- 2026-11-19 复检：`Get-PnpDevice` 中 12 个 `VID_041E&PID_3256` 节点（含 MEDIA 下的 “Sound BlasterX G6”）全部 `Present=False`；当前在线的 USB 音频只有 `VID_08BB&PID_2902`、`VID_3554&PID_FB3E`、`VID_3837&PID_3033`；`-WaitSeconds 90` 轮询无枚举，完整流程与 `-PreflightOnly` 均退出 2。近 6 小时系统日志无任何 `VID_041E` 插拔/枚举记录；本机为 Lenovo 90RC018GCP，`HypervisorPresent=True`。
- INF 复核：`oem33.inf` 的 G6 安装段 `[SB1770.Interface.Wave]` 使用 `SysFx1770.AddReg`，没有 `CTPKEY_GUID_SCOUT*`；唯一静态 Scout 表在 `SysFx1580.AddReg`，属于 `SB1580_Device`（PID_323A），值 `[2,3,3,-2,-2,0,3,4,3,2]`，不能代替 G6 实测。
- 无设备自测：扫频生成 + 合成 APO 10 段链恢复（0.6 dB 容差）；USBPcap 解析已有 `scout-01.pcap`，4 个 `5a 11` 帧、8 条目、`id 0 = 1.0` 校验通过。
- Crystalizer：`+0xF18` 固定二阶输出级接入（**2026-11-21 撤回，见 §16**）；`0x180195BD0` 的 256 点 Hann 窗与 `0x180195FD0` 的 128 点增益斜坡 `1/(1-0.5·sin²(πn/128))` 实现并对照；两缓冲重叠相加与 `+0x1F8` dry 叠加（权重全 1）、128 样本延迟已按 Ghidra 反编译补齐；探针新增窗口/斜坡、输出级、延迟与 dry 叠加检查。
- 仍未展开：Scout 实机取数需 G6 接入；`+0xF18` 一项已撤回（不存在该级）。

| 检查 | 结果 |
|---|---|
| `tools/scout-measure/verify-scout.cmd` | PASS（扫频/Analyzer/端到端 CLI + USBPcap 解析） |
| `run-g6-hardware.ps1`（含 `-PreflightOnly`） | 退出码 2：本机无 G6（PnP 无 VID_041E/PID_3256），预检按设计拒绝；完整流程同样停在预检 |
| `crystalizer_chain_ref.py` | PASS（含 Hann/斜坡、输出级、128 样本延迟与 dry 叠加） |
| `cargo test` | 28 passed（未改 Rust） |

## 11. 2026-11-19：十段增益 APO PID 静态映射

- 新增 `tools/apo-eq-coeff/extract-geq-pids.py`：用 `dnfile` 只读解析 `Creative.Platform.Devices.dll` 的 `CTTHXCommonPropKeys..cctor` IL，不加载/执行程序集。
- 结果：`GraphicEQ_Gain0..9` → PID 0–9（31–16 kHz）；Scout 全族共用 `CTPKEY_Scout_Guid`：0 `Enable`、1/2/3 SVM、4 `GEQ_Enable`、5 `PreampGain`、10–19 `GEQ_Gain0..9`、20 `Reverb_Enable`。
- 产物：`reference/g6-geq-pid-map.json` / `.md`。
- 新增 `tools/apo-eq-coeff/extract-soundcore-params.py`：导出 `CTSoundCore._etParam*` 枚举到 `reference/g6-soundcore-params.json` / `.md`。`_etParamGraphicEQ` 为 0 Enable、1 Preamp、2–11 Band0..9；`_etParamSmartVolume` 只有 0–3。旧抓包 “id 05 = SmartVol 74” 在枚举里无对应项，已降级为未验证推断。
- 边界：这些是 APO PID / 每特性 SoundCore 偏移，不是 `5a 11` 的 USB id；USB id↔频段仍必须逐段 +3 dB 抓包确认，旧 `scout-01.pcap` 不能替代。

| 检查 | 结果 |
|---|---|
| `extract-geq-pids.py --json` | 解出 20 个 Gain PID 与 Scout 全族，字段缺失即报错 |
| `extract-soundcore-params.py --json` | 解出 10 个目标枚举，缺项即报错 |

## 12. 2026-11-19：Rust 离线 WAV DSP 客户端

- 新增 `app/src/bin/sbc-dsp.rs`：读 WAV → EQ（Rust APO 精确系数）→ Crystalizer → Surround（仅 6 声道）→ Smart Volume → 写 float32 WAV。
- 新增 `reference/g6_chain_capi.h/.c`：不透明 C 句柄包装，内部按 128 帧分块，Surround 过渡斜坡与 DLL 块行为一致；Rust 不复制 C 结构体布局。
- `app/build.rs`：Windows 生成 `target/g6dsp-compile.bat`，`vcvars64` + `cl /TC /std:c11 /W4 /fp:strict` + `lib` 打包 `g6dsp.lib`，对象输出放在 `target/g6dsp-cc-<profile>`，避免非 ASCII 路径进入 cl 命令行；`G6_VCVARS` 可覆盖 vcvars 路径。非 Windows 用 `cc`。
- `app/src/eq_core.rs`：新增 `load_wav_f32` / `write_wav_f32`。
- 文档：`reference/sbc-dsp-cli.md`。

| 检查 | 结果 |
|---|---|
| 2ch 直通 | 输出与输入逐位一致 |
| 2ch Crystalizer 50 + SVM -20 | 全有限；RMS 0.0707 → 0.0506；最大差 0.316 |
| 6ch Surround 0x3f/50 | 全有限；FL/FR/BL/BR 最大差约 0.047–0.050，FC/LFE 不变 |
| EQ 预设 + Crystalizer + SVM | 全有限；最大差 0.255 |
| stereo 启用 Surround | 退出码 1，提示需要 6 声道，未创建输出 |
| `cargo test` | 31 passed（22 + 6 + 3，新增 3 个 FFI/链路测试） |

## 13. 2026-11-19：实时 WASAPI 桌面客户端

- 新增 `app/src/bin/sbc-live.rs`：eframe 桌面 GUI + `cpal` WASAPI 实时监听，麦克风 → EQ → Crystalizer → SVM → 耳机/扬声器。
- 采集/渲染两条流，中间 `rtrb` SPSC 环形缓冲；采集回调无堆分配、无锁，参数走原子量；`ChainHandle` 只移动进采集回调。
- 自动选 48/44.1/96/32 kHz 中双方都支持的 F32 配置；输入 1/2 声道转立体声，输出要求 2 声道；设备默认缓冲、共享模式。
- 新增 `--list-devices` 与 `--selftest N` 无界面模式；GUI 有设备选择、Start/Stop、Input gain、Crystalizer、SVM、EQ 预设和 RMS/峰值表。
- 文档：`reference/sbc-live.md`。

| 检查 | 结果 |
|---|---|
| `--list-devices` | 正确列出 WASAPI 输入 3 个、输出 3 个，均为 F32 |
| `--selftest 2`（UU 虚拟设备） | 输入 RMS 0.0006/峰值 0.0018；输出 RMS 0.0004/峰值 0.0013；underrun 2、overrun 0 |
| 同一自测 + Crystalizer 50 + SVM -20 | 链路正常，underrun 2、overrun 0 |
| 再加 EQ Acoustic | 输入 RMS 0.0001/峰值 0.0005；输出 RMS 0.0001/峰值 0.0006；underrun 2、overrun 0 |
| GUI 启动 | `timeout 5` 内持续运行，无 panic（rc=124） |
| 中文显示 | 启动时加载 `Deng.ttf` 后设备名正常显示，无方框 |
| release 构建 | `sbc-live` 通过 |
| 预缓冲 60 ms，UU 虚拟对 30 秒 | buffer 48 ms；underrun 0、overrun 0 |
| 同一对 + EQ Acoustic + Crystalizer 50 + SVM -20，10 秒 | buffer 48 ms；underrun 0、overrun 0 |
| USB PnP 麦克风 → MILTON 蓝牙耳机，10 秒 | buffer 170 ms；underrun 0、overrun 0 |

## 14. 2026-11-19：Windows x64 免安装包

- 新增 `dist/sbc-live-1.0.0-win64.zip`：`sbc-live.exe`、`sbc-dsp.exe`、`sbc-eq.exe`、`README.txt`、`run-sbc-live.cmd`、`SHA256SUMS.txt`。
- 静态 CRT：Rust `-C target-feature=+crt-static` + C 侧 `/MT`（`build.rs` 新增 `G6_STATIC_CRT`）；`dumpbin -dependents` 只剩系统 DLL，无 `vcruntime140.dll`/`msvcp140.dll`。
- 解压后校验 5 个文件的 SHA-256 全部一致；`sbc-live --list-devices`、`sbc-eq --list-eq-presets` 在包目录内直接运行通过。
- ZIP SHA-256：`cd96193762f7d1ebb1e94377cd5fb84f9c5b230ed24c9b0ddb40116eef097af8`（含 Bass/Dialog+ 参考实现重建）。
- 2026-11-19 重打包：`sbc-dsp` 换成本版（`--bass-mode apo-legacy` 警告文案），`sbc-live`/`sbc-eq` 不变；新 ZIP SHA-256 `fe3cff79442e5d1c328ff3942b304e8c285022d929d9dc635c9d70a56b29c76b`，桌面副本同步替换。

| 检查 | 结果 |
|---|---|
| 静态 CRT 构建 | `sbc-live/sbc-dsp/sbc-eq` 全部成功 |
| 依赖检查 | 仅系统 DLL（kernel32/user32/ole32/OPENGL32 等） |
| 包内 SHA256SUMS | 5/5 一致 |
| 解压后运行 | `--list-devices`、`--list-eq-presets` 正常 |

## 15. 2026-11-19：Bass / Dialog+ 逆向边界与参考实现

- 逆向状态：Bass 已到 vtable/处理函数级——`CBassManagementEfxMod` vftable `0x180188560`（+0x18 init `FUN_18003e420`、+0x40 process `FUN_18003de90`）、`CBassManager` process `FUN_1800cc050`、10-tap FIR `FUN_1800cc560`（系数 `[0,0,.05,.1,.15,.2,.2,.15,.1,.05]`）、增益映射 `FUN_1800cb5e0`、限幅 `FUN_1800cc310`、混合 `FUN_1800cb9e0`；Dialog+ 在 `KSUSBAPO64.dll` 无字符串，`KSUSBSPI64.dll` 只有 `THXDialogueAdvantage_Amount/isEnabled` 参数处理器（`FUN_0044c1ac/FUN_0044c62c`）和 `0x4078F0` 附近处理器 vtable，未找到 DSP 循环。
- 参考实现：新增 `reference/g6_bass.c/h`（Missing Fundamental）与 `reference/g6_dialog.c/h`（2–4 kHz + 轻量门），接进 `g6_chain_capi`、`sbc-dsp`、`sbc-live`；链路顺序 EQ→Crystalizer→Bass→Dialog→Surround→SVM。
- 实测：Bass 100 / 60 Hz 输入 → 120 Hz -26.4 dB、180 Hz -39.9 dB；Dialog 100 / 1k+3k → 3 kHz +5.8 dB；实时 5 秒 Bass 100 + Dialog 100，buffer 48 ms，0 underrun。
- APO 链近似：`g6_bass.c` 增加 `mode=1`（10-tap FIR + 三次整形 + 20 Hz DC 阻断），`sbc-dsp --bass-mode apo`；8× pre-gain 为听感校准，60 Hz 实测 180 Hz -20.8 dB、300 Hz -30.5 dB、120 Hz -45.8 dB，峰值 0.464。默认仍为 reference。
- **2026-11-19 修正**：继续反编译确认 `FUN_1800cc560` 的 10-tap FIR 与 `FUN_1800cb5e0` 的三次多项式作用在**增益包络**上，不是音频；`CBassManagementEfxMod` 是软拐点限幅/动态链（包络→20log10→限幅→FIR 平滑→增益映射→乘回音频）。`FUN_180089780` 是 bass management 分频/路由核心：每声道一阶高通（`H(z)=c(z⁻¹-1)/(1+a2z⁻¹)`，`c=-2/(k+2)`、`a2=(k-2)/(k+2)`、`k=3π*cutoff/fs`），卫星声道去低频；同时把**原始** 6 声道以 -15 dB 汇总到 LFE/sub。不是低音提升，也不是 Missing Fundamental 谐波。`mode=1` 因此标记为 legacy 误读，仅保留历史对比。（2026-11-21 补充：同一个和里，LFE（声道 3）另以 ×0.56234133（-5 dB）直接进入；其余声道取**原始**样本 ×0.17782794（-15 dB），再各自覆盖为一阶高通输出。见 §23。）
- Dialog 补充：`CVocalSuppressorEfxMod` vftable `0x18018de98`、init `0x180077b20`、process `0x180077860` 已解；它是 4 段可交叉淡化滤波器组（属性 GUID `{1ee40950-a46c-45eb-a50c-f1fd995c0166}`，index 0..7），不是 Dialog+ 的 2–4 kHz 对白增强，未找到 `THXDialogueAdvantage` 的实际 DSP。32/64 位 APO 的 EfxMod 类名列表一致（118 个，均无 Dialog 类）；`KSUSBDGFX64.dll`/`KSUSBWrap64.dll` 只是 APO 包装器（IAudioSystemEffects / DaisyWrapEFX），无 DSP。
- THX Bass 补充：`CBassMgmtEfxMod` vftable `0x18018cb38` 是另一条低音管理链：`FUN_180067890` 解析 `bass` 配置文件的 `crossover`/`boostlevel`/`virtuallevel`，`FUN_1800a9670` 做双二阶交叉滤波 + 128 帧参数淡化；它做分频/路由，不是 Missing Fundamental 谐波合成。
- 文档：`reference/g6-bass-dialog.md`、`reference/g6-bass-dialog-vtable.md`；PDF 对照更新 `reference/g6-algorithm-review.md`。

| 检查 | 结果 |
|---|---|
| `sbc-dsp --bass 100` | 输出有限，峰值 0.524，谐波生成符合预期 |
| `sbc-dsp --bass 100 --bass-mode apo` | 输出有限，峰值 0.464，180/300 Hz 谐波 |
| `sbc-dsp --dialog 100` | 输出有限，峰值 0.674，3 kHz 提升 |
| `sbc-live --selftest` + Bass/Dialog | 5 秒 0 underrun / 0 overrun |





## 16. 2026-11-21：Smart Volume 代码级移植与 Crystalizer 假级撤回

无硬件、纯静态取证 + 重新对拍。全部结论来自 `KSUSBAPO64.dll` 1.2.109.0 的反编译与镜像常量读取，
未运行原厂代码。

### 16.1 Smart Volume 重写

- 对象是 `CPlugAGCCompressor`：构造 `0x180092810`、复位 `0x180092bc0`、检测器系数 `0x180092a50`、
  参数更新 `0x180092da0`、样本循环 `0x180092f30`、启用/旁路 `0x180093420`、改延迟 `0x1800935c0`。
- 逐行重写 `reference/g6_svm.c`：单检测器全带压缩，**20 ms 前瞻**（此前写成 30 样本）、
  两抽头延迟交叉淡化、三级平滑（gain1 / attack 平滑 gain2 / 10 ms makeup）、启用时 `10^1.1 = +22 dB` 补偿增益。
- 数学函数定性有据：`0x1800dc0b0` 是 log10（引用常量 `0x1801a0a00 = 0.43429449 = log10(e)`），
  `0x1800d96d0` 是 `10^x`；因此 `+0x4b98 = -6.2` 对应 -62 dBFS 能量、`10^1.1` 对应 +22 dB，都是整数 dB 设计值。
- 旧 `set_level` 声称的 `(level_db-12)/10` 来自 `0x180093710`，但该地址**不是函数**；`+0x4b98`
  与 `+0x4b8c` 在构造之外没有任何写入点，DLL 没有电平旋钮。旧映射保留为 legacy，
  新增 `g6_svm_set_threshold_dbfs`。
- PDF 的"多频段 AGC + 分频段 DRC + 峰值限幅"与 APO 不符：检测器每声道只有一个一阶高通 + 能量平滑，
  跨声道取最大值，增益律是单一 log 域直线，没有独立限幅器。

### 16.2 Crystalizer `+0xF18` 撤回

- 全量扫描 `.text` 位移 `0xf14/0xf18/0xf1c/0xf20/0xf24`：`0xf18` 与 `0xf14`、`0xf24` 均 0 次命中。
- `0x18008EC00` 的 `+0x2A8` 是全局 DSP 对象的 **vtable 槽位**（`(**code)(*(PTR_DAT_1801f1e80 + 0x2a8))`），
  不是对象字段。`0x18008E920` 的真实结构是每声道三阶状态 `+0xe50+ch*0x18`、二阶状态 `+0xe98+ch*0x28`，
  最后是与原始块的 ramp 混合（vtable `+0x180`）。
- 已从 `g6_crystalizer.c/h` 删除 `output_filter` 与 `g6_crystalizer_set_output_filter`，探针改输出
  `output_stage none`，参考脚本据此校验。

### 16.3 新增工具

- `tools/apo-eq-coeff/ghidra_scripts/G6Probe.java`：scan（位移扫描）/ dump（反编译）/ vtable / sym / disasm 五模式。
- `tools/apo-eq-coeff/pe_read.py`：只读 PE 映像，按 VA 取 qword/dword/float（不加载、不执行 DLL）。
- `tools/apo-eq-coeff/run_ghidra_probe*.cmd`：一次导入、多次 `-process` 复用已分析工程。

| 检查 | 结果 |
|---|---|
| `verify_svm.cmd` | PASS（14 个常量位级一致、6144 帧增益轨迹、0.08 s 交叉淡化在第 3840 帧归零、旁路） |
| `verify_crystalizer.cmd` | PASS（`output_stage none`） |
| `cargo test` | 31 passed（22 + 6 + 3） |
| `sbc-dsp` 端到端 | 48000 Hz / 2ch / 131072 帧，peak 0.25，rms 0.000488，全有限 |
| `--svm-delay-ms 5 / 50` | 均正常，5 ms 被钳到 30 样本 |

### 16.4 重新打包

- `dist/sbc-live-1.0.1-win64.zip`：`sbc-live`/`sbc-dsp`/`sbc-eq` 全部按本版源码重建，静态 CRT
  （`dumpbin /dependents` 对 `sbc-dsp` 只剩 `KERNEL32.dll`、`ntdll.dll`、`api-ms-win-core-synch-l1-2-0.dll`）。
- 解压后 `SHA256SUMS.txt` 5/5 一致；包内 `sbc-live --list-devices` 与 `sbc-eq --list-eq-presets`（43 条）正常。
- ZIP SHA-256：`410870b2e52011f7564d914d8f4b70b02fe8968ca910cb907a1f4b4fc6214b24`。
- 1.0.0 包保留作为历史，不再更新。

详细依据：[g6-svm-exact-port.md](g6-svm-exact-port.md)。

## 17. 2026-11-21：SBX / Scout / Direct 模式语义

纯软件，无硬件依赖。此前 `sbc-live` 把五个效果当作互不相干的开关，这与产品语义不符。

### 17.1 数据

- 新增 `app/src/sbx_modes.rs`，以 `reference/g6-product-data.json` 为唯一来源解析 23 个工厂 profile。
- 关键发现：profile 存的是**每个效果的 Enable + Level**，不是只有电平。Gaming 关闭 Bass/SVM/Dialog+，
  Movie 打开 SVM(0.40)，RPG 同时打开 Bass(0.20) 与 Dialog+(0.15)。只写电平会得到错误状态。
- 23/23：`SBXMaster.Enable` = on、`Scout.Enable` = off、`Bass.XOver` = 80、`SVM.Mode` = 0。
  即互斥是运行时开关，profile 数据里没有 Scout 态；`SVM.Mode` 一致为 0，与 §16 的"没有模式曲线"吻合。
- Speaker 与 Headphone 只有 2 个 profile 有差异，且都不在五个效果字段上，因此客户端只读 speaker 一份。

### 17.2 行为

- `Mode::{Direct, Sbx, Scout}`；`EffectLevels::bypass_all()` = `direct || scout || !sbx_master`。
- `sbc-live`：顶部模式行 + profile 下拉；Direct/Scout 下效果控件禁用并清空 EQ 勾选；选中 profile 立即装载。
- `sbc-dsp`：`--mode direct|sbx|scout`、`--sbx-profile NAME`；冲突组合直接报错。
- 两处刻意近似并已在文档标注：profile 的 SVM 电平用 `-50 + 0.3*level` 映射到既有 dB 滑杆；
  profile 不带宽高比 mask，6 声道固定 `0x3f`。

### 17.3 顺带修掉的缺陷

`g6_chain_create` 之前直接沿用 `g6_svm_init` 的构造函数状态（`0x180092810` 在开机时把延迟从
2400 淡到 960），导致 `g6_svm_active()` 永远为真、链条在**没开 SVM 时也处理音频**。
新增 `g6_svm_settle()` 让新链进入稳态；探针新增 `settled` 检查。

| 检查 | 结果 |
|---|---|
| `cargo test` | 38 passed（29 lib + 6 桌面 + 3 链） |
| `sbc-dsp --mode direct` vs 输入 | **逐字节一致**（1048576 字节） |
| `--sbx-profile Gaming` / `movie` | 与输入及彼此均有差异（Movie 打开 SVM） |
| 6 声道 + `--sbx-profile Gaming` | 走 Surround 分支，无跳过警告 |
| 错误路径 | `--mode sbx` 缺 profile、Direct/Scout + 效果参数、未知 profile/mode 全部报错 |
| `verify_svm.cmd` / `verify_crystalizer.cmd` / `verify_surround.cmd` | 全部 PASS |
| `sbc-live` GUI | 6 秒无 panic（rc=124）；`--selftest 1` 正常 |

### 17.4 重新打包

- `dist/sbc-live-1.0.2-win64.zip`：静态 CRT 重建，解压后 `SHA256SUMS.txt` 5/5 一致，
  包内 `sbc-dsp --help` 已含 `--mode` / `--sbx-profile`。
- ZIP SHA-256：`554d2d74f8ff56952304c7b228d133a562cb78ca2d03543c48b7afe01acf291c`。
- 包内 README 增加了模式说明与两条示例命令。

详细说明：[sbx-modes.md](sbx-modes.md)。

## 18. 2026-11-21：`0x180096fe0` 基类构造与格式协商

纯静态取证，未运行原厂代码，未改动 `C:\Windows\System32` 下任何文件。

- `0x180096fe0` 是 `CPlug` 基类构造：先把源对象 `source+0x14..0x20` 的 4 dword 格式拷到
  `this+0x10..0x1c`，按 4 个实参协商，再把结果镜像到 `this+0x34..0x40`。
  全镜像只有两个调用点：CPlugReverb `0x180091b1f` 传 `(22050, 96000, 5, 1)`，
  CPlugAGCCompressor `0x180092842` 传 `(200, 48000, 5, 2)`。
- 内部格式布局由 `0x180098470`（内部格式 → `WAVEFORMATEXTENSIBLE`）确定：
  `{采样率, 样本类型, 声道数, 声道掩码}`。样本类型表 `0x180198f28` 是 u16[6]
  `{0,1,2,3,4,4}` 字节/样本：1–4 配 PCM GUID（`0x180191558`，8/16/24/32 位），
  5 配 IEEE float GUID（`0x180185498`）。因此 `a5 = 5` 就是 float32。
- 旧记的「`+0x14` 是声道数」不成立：声道在 `+0x18`，`0x180092f30` 的输入/输出循环上限
  正是 `+0x18` / `+0x3c`，`+0x1c` / `+0x40` 是 `& 0x7fffffff` 的声道掩码。
- `0x180098120` 返回 true 表示「4 个字段任一不等」。构造里用它决定是否调用
  `source->SetFormat`。本机 `CDummySource` 的格式是 `{48000, 5, 2, 0}`，与 AGC 的请求全等，
  所以**默认路径不触发协商**；Reverb 请求 `{48000, 5, 1, 0}`，第三字段不等，会请求单声道。
- `minRate` 只在 `源率 < minRate` 时成为请求值，DSP 侧没有对应下限；AGC 的 200 Hz
  在正常源上不可达，业务理由未取证。
- 工具：`G6Probe.java` 新增 `data <addr>:<len>` 模式（hex + dword 视图），
  本轮用它读出样本类型表与两个 GUID；`xref` 用于确认 `0x180098470` 的调用者与
  `0x180096fe0` 的调用点数量。

详细依据：[g6-svm-exact-port.md](g6-svm-exact-port.md) §8。

## 19. 2026-11-21：Bass 函数级补完（限幅器常量、mixer 状态、`CBassBoost` 定案）

纯静态取证（Ghidra 12.1.4，`KSUSBAPO64.dll` 1.2.109.0）。未运行原厂代码，
未改动 `C:\Windows\System32` 下任何文件；未做实时回环（本机无 G6 设备）。

- 限幅器不是独立对象：`0x1800cbdb0` 的 `this` 是 `CBassManagementEfxMod+0x188`，
  由 `0x18003eae0` 在按块大小预分配后以 `{1, (float)采样率, 帧数}` 调用。
- 限幅器常量：`+0x2c = 0.5`（斜率）、`+0x1bf0 = -4.5`（拐点 dB）、
  `+0x28 = 1 - exp(-2432 / (rate*250/[2]))`；`0x1800e1740` 是内联 `exp()`
  （64 项 `2^(k/64)` 三表）。`[2]` = 8（<88.2 kHz）/ 16（≥88.2 kHz），
  `+0x20 = frames/[2]` 且断言要求整除；四档采样率折算后时间常数同为 102.8 ms。
- 两个 mixer（`CBassBoostMixerSmallSpeakers` / `CBassBoostMixerLFE`）是 0x48 复合对象：
  `[0]` mixer vtable、`[+8]` `CHspIIR2State`、`[+0x30]` `CHspLimiter1State`，
  在 init 里手写，没有构造函数；`0x18003de90` 把 `+0x168` 作为 `0x180088ff0`
  的第一实参。所以 mixer 不是纯路由，但 0x48 字节里没有系数。
- `CBassBoost` 没有内核：构造 `0x180089470` 只写 vtable 并构造 12 × 0x28 数组；
  `CBassManager` 构造 `0x1800894e0` 把它放在 `+0x60`（`0x60 + 0x1e8 = 0x248`，
  与紧邻的 `memset(+0x248, 0, 0xc0)` 对齐）。处理槽 `0x180089b80` 走
  `0x180088d80(+0x60, +0x34, block)`，逐声道调全局 DSP 对象 `vt[+0x288]`。
- 两处修正：属性描述符每条第 `0x14` 字节（`PROPERTYKEY`，9 条 = `0xb4`），不是 `0x18`；
  `0x18004d1a0` 的 `+0x1bf0 = 2` 属于采样率转换器，与限幅器的 `-4.5f` 同偏移不同类。
- `reference/g6_bass.c` 只加注释：mode 0 明确标为 PDF 方向参考，不是 APO 移植。

| 项 | 状态 |
|---|---|
| 限幅器初值/公式 | 已解 |
| mixer 实例化与状态布局 | 已解 |
| `CBassBoost` 无内核 | 已解 |
| `CBassManager` 构造常量 | 已解 |
| 全局 DSP `vt[+0x288]` 语义 | 未解 |
| `FUN_180088ff0` 如何消费 mixer 内嵌状态 | 未解 |
| LFE 变体处理槽 | 已解（E42–E44，§23） |
| 实机扫频/听感 | 未执行 |

§20 更新：前三项已解（单例身份、七步流水线、两条低音链频响）。
§23 更新：LFE 变体的子类映射已解（E42–E44）。

详细依据：[g6-bass-dialog-vtable.md](g6-bass-dialog-vtable.md) §8。

## 20. 2026-11-21：Bass 全局 DSP 单例与两条低音链频响

纯静态取证（Ghidra 12.1.4，`KSUSBAPO64.dll` 1.2.109.0）。未运行原厂代码，
未改动 `C:\Windows\System32` 下任何文件；未做实时回环（本机无 G6 设备）。

### 20.1 全局 DSP 是 `CStaticHspSystem`

E13：`0x1801f1e80` 是指向静态对象 `0x180207c58` 的全局指针。对象 8 字节，只存 vptr。
E14：`0x180009a40` 按两个 CPU 特性字节选表；E15 的 RTTI 给出类名：

| 条件 | vftable | RTTI 类名 |
|---|---|---|
| `byte[0x180207cea] != 0` | `0x180195138` | `CStaticHspSystemSSE2` |
| 否则 `byte[0x180207ce9] != 0` | `0x180194cd8` | `CStaticHspSystemSSE` |
| 否则 | `0x180194878` | `CStaticHspSystem` |

对象没有系数与历史缓冲；状态全部由调用方传入。[推断] “Static” 指这张按 CPU
选一次的 DSP 原语表，不是有状态滤波器实例。

### 20.2 三处纠错

| 项 | 旧记 | 现行（E16/E23） |
|---|---|---|
| `+0x450` 指向 | `0x18014ab90` | `0x180088af0`（包络限幅器） |

2026-11-21 二次校正（§23）：`0x180191738` 与 `0x18019173c` 的首次“修正”把两者写反。
按位实读：`0x180191738 = 0.17782794`（-15 dB）、`0x18019173c = 0.56234133`（-5 dB）。
本节早先版本声称前者是 -5 dB，该说法已撤回。

`0x18014ab90` 实际在 `+0x448`，低音链路不调用；语义已于 §22 解出。

### 20.3 `FUN_180088ff0` 七步流水线（E17）

门控为 `(inMask & 8) == 0 && (int)inMask >= 0 && bypass != 0`。随后：
清零 `dst` → `inter = inMask & outMask`（为 0 则取 3）→ 非交声道以 -15 dB
累加进 `dst` → `+0x2a8` 二阶 IIR（转置型）→ `+0x450` 峰值包络限幅器 →
全体 +10.5 dB → 交声道相加回目标声道。交声道净增益 -4.5 dB。

限幅器 `0x180088af0` 的递推：

```c
e = fmaxf(fabsf(x) - env, 0.0f);
env = env*rel + e*atk;
if (thr < env) x = thr*x/env;
```

### 20.4 `CBassBoost` 系数与频响（E18/E19）

`FUN_180089c40` 写 12 个槽，每槽写 `a1 / b0 / b1`：

```c
K  = 1.0f / tanf((fc/fs) * PI);
p  = 1.0f / (1.0f + K);
a1 = (1.0f - K) * p;
b0 = p * (2.0f*g - 1.0f);
b1 = p * K;
```

`+0x288` 的递推等价于双实极点二阶节：

```
H(z) = b1*((1+b0) + (a1-1)z^-1 - (a1+b0)z^-2) / (1 + 2*a1 z^-1 + a1^2 z^-2)
```

`fc = 100`、`fs = 48000` 时，`g = 0.5` 使 `b0 = 0`，退化为 100 Hz 一阶高通
（-3.01 dB）；`g = 2` 时 100 Hz 为 +6.28 dB、150 Hz 为 +5.90 dB。
默认 `g = 1.0`、`fc = 100`。

### 20.5 `FUN_180089780` 的贝斯管理链（E25/E26）

信号流：LFE×(-5 dB) 加其余声道各自的**原始**样本 ×(-15 dB)（声道随后写回一阶高通输出）→ 二阶低通 →
`× [+0x5c]` → 声道 3。

一阶高通 `K = 3π fc/fs`，`a = -2/(K+2)`、`c = (K-2)/(K+2)`。
`fc = 100` 时 -3 dB 在 150 Hz（1.5 倍 fc）；20 Hz 为 -17.58 dB，
100 Hz 为 -5.12 dB。二阶低通的 `fc` 同为 100 Hz：8 kHz 为 -2.56 dB，
16 kHz 为 -12.21 dB。

`fc` 由属性 2/4 写 `+0x330/+0x33c`，`g` 由属性 1 写 `+0x334`，
输出增益由属性 5 写 `+0x340`。

### 20.6 mixer 系数：init 清零，`+0xe0` 写入（E24/E27/E28）

`0x18003e420` 建 0x48 的 mixer，只写两个 vptr，并清零 `+0x10..+0x28`
与 `+0x38..+0x44`。

2026-11-21 修正（§21）：`0x18003e420` 末尾调用 vtable `+0xe0`，即
`FUN_18003ef90`，由它调用 `FUN_180088cf0` / `FUN_180088470` 写入 limiter 与
IIR 系数；格式协商 `+0xd8`（`0x18003ec30`）也写同一组系数。
七步流水线第 5–7 步在正常路径上产生信号，不是空转。

### 20.7 覆盖矩阵

| 项 | 状态 |
|---|---|
| 全局单例身份与三实现 | 已解（E13–E16） |
| `FUN_180088ff0` 七步 | 已解（E17） |
| `+0x288` 递推、系数公式、频响 | 已解（E18/E19） |
| `+0x450` 包络限幅器 | 已解（E23） |
| 贝斯管理两条滤波器与频响 | 已解（E25/E26） |
| mixer 系数写入者 | 已解（E24/E27/E28） |
| `+0x448`（`0x18014ab90`）语义 | 已解（E31–E41，§22） |
| LFE 变体绑定（`0x180088e60` / `0x1800891f0`） | 已解（E42–E44，§23）：LFE mixer 归 `CCMSSEfxMod` |
| `+0x1bec = 250.0` 读取点 | 未解 |
| 常量池其余项（`0x1801916b4..e8`、`0x180191830..`） | 归属未解；`0x1801916ec` 已用于 limiter |
| 实机听感/扫频回环 | 未执行（无设备） |
| `frames % 4 != 0` 越界行为 | 推断，未验证 |

详细依据：[g6-bass-dialog-vtable.md](g6-bass-dialog-vtable.md) §9–§12。

## 21. 2026-11-21：mixer 系数写入者已定位（撤回 §20.6 的空转结论）

纯静态取证（Ghidra 12.1.4，`KSUSBAPO64.dll` 1.2.109.0）。未运行原厂代码，
未改动 `C:\Windows\System32` 下任何文件；未做实时回环（本机无 G6 设备）。

### 21.1 写入者是 `+0xd8/+0xe0`

E27：`CBassManagementEfxMod::vftable` 基址 `0x180188560`。
`0x18003e420` 成功路径末尾执行 `CALL [RAX+0xe0]`（`0x18003e7fc`），
槽 `0x180188640` = `0x18003ef90`。`FUN_18003ef90` 在参数变化时执行
`CALL [RAX+0xd8]`（`0x18003f12d`），槽 `0x180188638` = `0x18003ec30`。

E28：两个函数都把 `EfxMod+0x168` 当 mixer，并写同一组系数：

```c
FUN_180088cf0(mixer + 0x30, 10.0f, 50.0f, 0.17782794f, (float)fs);
FUN_180088470(mixer + 8, (float)fc / (float)fs);
```

`FUN_18003e420` 末尾的 `+0xe0` 调用带 `force=1`；constructor 里
`+0x128` 与 `+0x12c` 同为 100，因此正常 init 写入 `fc/fs = 100/48000`。
只有绕过 `+0xe0` 的构造路径才保留全 0。

### 21.2 limiter 与 IIR 的具体值

E29：`FUN_180088cf0` 写 `cfg = mixer+0x30`：

```c
cfg[0x08] = 0.03f / ((fs / 48000.0f) * 10.0f);
cfg[0x0c] = 1.0f - 0.01f / ((fs / 48000.0f) * 50.0f);
cfg[0x10] = 0.17782794f;
```

绝对偏移 `mixer+0x38/+0x3c/+0x40`；`+0x44` 是运行时包络。
48 kHz 时 attack = 0.003、release = 0.9998、threshold = -15 dB。
[推断] 对应时间常数约 6.94 ms 与 104.17 ms，未实测。

E30：`FUN_180088470` 写 `cfg = mixer+8`：

```c
w  = 2*PI*(fc/fs);
A  = 0.5f*(1 - cosf(w)) / (1 + cosf(w));
p  = A + 0.5f;
s  = sinf(w)*p;
k  = (s - p)*0.25f;
cfg[0x08] = 2*s;          /* -a1 */
cfg[0x0c] = -2*A;         /* -a2 */
cfg[0x10] = 0.5f*(s-p);   /* b0 */
cfg[0x14] = (s-p);        /* b1 */
cfg[0x18] = 0.5f*(s-p);   /* b2 */
```

`FUN_1801469e0`（`+0x2a8`）的递推：

```c
v_n = cfg[0x08]*v_{n-1} + x_n + cfg[0x0c]*v_{n-2};
y_n = cfg[0x14]*v_{n-1} + cfg[0x10]*v_n + cfg[0x18]*v_{n-2};
```

与 §11.3 的低通同式。`fc = 100`、`fs = 48000` 时
`b0 = b2 = -0.24673817`、`b1 = -0.49347634`、`a1 = -0.01309016`、
`a2 = 0.000042838`。

### 21.3 结论与覆盖

- §20.6 的“mixer 空转、写入者未定位”撤回。
- `+0x2a8` 是带系数的二阶低通；`+0x450` 是带参数的包络限幅器。
- 七步流水线第 5–7 步在正常路径上产生可听信号。

| 项 | 状态 |
|---|---|
| mixer 系数写入者 | 已解（E27/E28） |
| limiter 参数公式与常量 | 已解（E29） |
| `+0x2a8` 低通系数与布局 | 已解（E30） |
| 正常 init 是否写系数 | 已解：`+0xe0` 在 `0x18003e420` 末尾调用 |
| `+0x448`（`0x18014ab90`） | 已解（E31–E41，§22） |
| LFE 变体绑定 | 已解（E42–E44，§23） |
| 实机听感/扫频 | 未执行（无设备） |

详细依据：[g6-bass-dialog-vtable.md](g6-bass-dialog-vtable.md) §12。

## 22. 2026-11-21：`+0x448` 是归一化幅度比检测器

`+0x448` 一直记为“未解码、低音链路不调用”。函数已读到底，调用点也已定位。
结论：它是**逐样本的归一化幅度比检测器**，不是限幅器。
详细推导见 [g6-bass-dialog-vtable.md](g6-bass-dialog-vtable.md) §13。

### 22.1 它是 HSP 原语表的虚方法（E33）

七张 vtable 以 `0x460` 步长连续排列，每张 138 槽（`0x450` 字节），
RTTI 类名逐个读出：`CHspSystem`、`CHspSystemSSE`、`CHspSystemSSE2`、
`CStaticHspSystem`、`CStaticHspSystemSSE`、`CStaticHspSystemSSE2`、`CSensHSP`。

| `+0x448` | 使用它的表 |
|---|---|
| `0x1801444a0`（标量） | `CHspSystem`、`CStaticHspSystem`、`CSensHSP` |
| `0x18014ab90`（SSE2） | `CHspSystemSSE`、`CHspSystemSSE2`、`CStaticHspSystemSSE`、`CStaticHspSystemSSE2` |

§20.1 的运行时分派只在后三张 `CStaticHspSystem*` 表里选一个，
所以运行时走向是 `0x1801444a0`（通用）或 `0x18014ab90`（SSE/SSE2）。

### 22.2 唯一调用点（E34）

```
18012cdb0  CALL qword ptr [R10 + 0x448]      ; FUN_18012cd40
```

全镜像 `scan 0x448` 的 18 处命中里只有这一条是间接调用。
链条：`0x180196bc0` 的 5 项函数指针表 → `FUN_1800963f0`（`0x18009652f`）→
`FUN_18012ac10`（HSP 八通道渲染）→ `FUN_18012cd40`（`0x18012b02b`）→ `[vt+0x448]`。

### 22.3 数学（E31/E32）

```c
for (i = 0; i < count/2; i++) {
    p3[i] = (1.0f - mix) * p7[i] + mix * p3[i];
    p4[i] = (1.0f - mix) * p8[i] + mix * p4[i];
    p5[i] = (1.0f - mix) * p9[i] + mix * p5[i];
    p6[i] = (1.0f - mix) * p10[i] + mix * p6[i];
    if (p3[i] < thr || p4[i] < thr) {
        out[i] = 0.0f;
    } else {
        out[i] = 1.0f - sqrtf(p5[i]*p5[i] + p6[i]*p6[i])
                       / sqrtf(p3[i] * p4[i]);
    }
}
```

`0x1801865cc = 1.0f`。`mix` 来自 job 的 `+0x58`，`thr` 来自 `+0x64`，
`count` 来自 `+0x74`，`out` 是 `[hsp+0x3c9e8]`。

SSE2 版把前三段固定成 `64 × 8 = 512` 个 float，只有门限尾循环用 `count>>1`。
`count` 由 `FUN_180129c40` 写入 `[hsp+0x3633c] = 2 * min(n, 512)`，
所以 `n >= 512` 时 `count>>1 == 512`，与定长块一致（§22.6）。

### 22.4 与 `+0x450` 的分工

| 维度 | `+0x448` | `+0x450` |
|---|---|---|
| 输出 | `1 - 归一化幅度比`，可为负 | 限幅后的样本 |
| 状态 | 无，全部实参传入 | `CHspLimiter1State` 的 env |
| 形态 | 检测器 | 包络限幅器 |

`+0x448` 没有 attack/release 递推，也没有 `x = thr*x/env` 的增益缩减，
所以它不是限幅器。低音七步流水线只调 `+0x450`，与本节无关。

### 22.5 下游与覆盖

`[vt+0x448]` 返回后立刻调 `FUN_18012c7d0`（E35）：它把 `out[i]` 送进查表整形器
（表在 `job+0x28`），再按 `job+0x5c/+0x60` 的交叉淡化系数混进 `job+0x30` 的累加缓冲。
所以 `+0x448` 的输出是整形器的输入信号，不是最终增益。

### 22.6 job 的构造（E38）

`FUN_18012cbd0(job_out, params, count, fs)` 构造并复位 job：

- `job = *(void**)(params + 0x58)`；job 内嵌在宿主对象 `hsp+0x366c0`。
- 复制 `params[0x00..0x50]` 的 11 个 qword 到 `job`。
- 写 `job+0x74 = count`、`job+0x70 = fs`。
- 写 `job+0x68 = 0.5f`、`job+0x64 = 0.0016070808f`、`job+0x6c = 0.001f`。
- 写三个平滑系数 `job+0x58/0x5c/0x60 = c^(24000/fs)`，
  `c = 0.6254270 / 0.6739930 / 0.9294860`；`powf` 在 `0x1800d96d0`。
- 把 `acc` 与 p3..p6 清零，长度 `count>>1`。

调用点 `FUN_180129c40` 的 `0x18012a1ab` 传入
`hsp+0x36378`（job 槽）、`hsp+0x36508`（参数块）、`[hsp+0x3633c]`、`[hsp+0x36324]`。
采样率在这里被夹到 `[24000, 48000]`。

### 22.7 缓冲与表（E39/E41）

宿主类是 `CCMSS3UpmixComponent`（`FUN_1801293e0` 写入 vtable）。
参数块由 `FUN_18012a3f0` 写入，八个缓冲都从对象内存储切出，每个 `0x810` 字节
（516 float，实用 512），双声道区段相邻，四个阵列相差 `0x1030`。

| 角色 | 声道 0 | 声道 1 |
|---|---|---|
| 输入对 A（p7/p8） | `hsp+0x368d0` | `hsp+0x370e0` |
| 输入对 B（p9/p10） | `hsp+0x37900` | `hsp+0x38110` |
| A 的平滑状态（p3/p4） | `hsp+0x38930` | `hsp+0x39140` |
| B 的平滑状态（p5/p6） | `hsp+0x39960` | `hsp+0x3a170` |

`ctrl = hsp+0x3a990`，`acc = hsp+0x3b9c0`，`out = hsp+0x3c1d0`，表 = `0x1801f9e80`。
所以比值读作 `out = 1 - |B| / sqrt(A0*A1)`。
[未解] A、B 对应哪两路扬声器信号。

表 `0x1801f9e80` 恰好 202 项（0.0026019 → 0.8333692），索引是
`trunc(out * 201.0f)`，用 `1.5*2^52` 魔数实现向零截断。

| 项 | 状态 |
|---|---|
| 函数语义 | 已解（E31/E32） |
| 两个实现等价 | 已解（E31/E32/E36） |
| 所属类（七张表） | 已解（E33） |
| 唯一调用点 | 已解（E34） |
| 参数与 job 布局 | 已解（E34） |
| 下游整形器 | 已解（E35） |
| job 构造与参数常量 | 已解（E38） |
| 八个缓冲的身份 | 已解（E39） |
| 平滑系数采样率换算 | 已解（E40） |
| 整形器索引与表边界 | 已解（E41） |
| A/B 的扬声器语义 | 未解 |
| `ctrl` 写入者 | 未解 |
| 实机听感 | 未执行（无设备） |

详细依据：[g6-bass-dialog-vtable.md](g6-bass-dialog-vtable.md) §13。

## 23. 2026-11-21：LFE mixer 归 `CCMSSEfxMod`，两个增益常量二次校正

`FUN_180088e60` 长期记作“LFE 变体、绑定未定”。本节把写入者、vtable 与调用点对齐。
结论：`CBassBoostMixerLFE` 的宿主是 `CCMSSEfxMod`（CMSS/SBX 效果），
不是 `CBassManagementEfxMod`。

### 23.1 唯一写入者（E42）

| mixer 类 | vftable | COL | 唯一写入函数 | 宿主槽 |
|---|---|---|---|---|
| `CBassBoostMixerSmallSpeakers` | `0x1801886e0` | `0x1801c0880` | `FUN_18003e420` | `CBassManagementEfxMod+0x168` |
| `CBassBoostMixerLFE` | `0x180188bd0` | `0x1801c0c28` | `FUN_1800414b0` | `CCMSSEfxMod+0x1d0` |

`xref` 显示每个 vftable 只有 2 条引用（`LEA` + 存指针）。两个写入函数的构造一致：
`LocalAlloc(0x40, 0x48)`、`+0x08 ← 0x1801886b0`、`+0x30 ← 0x1801886c0`、其余清零。
差别只有首 qword（`0x1801886e0` 对 `0x180188bd0`）与写入宿主的位置。

RTTI 实读：`0x1801c0880` 是 `CBassBoostMixerSmallSpeakers`、
`0x1801c0c28` 是 `CBassBoostMixerLFE`、`0x1801c0950` 是 `CBassManagementEfxMod`、
`0x1801c0ca8` 是 `CCMSSEfxMod`。

### 23.2 两个宿主（E43）

| 类 | vftable | `+0x18` init | `+0x40` process |
|---|---|---|---|
| `CBassManagementEfxMod` | `0x180188560`（COL `0x1801c0950`） | `0x18003e420` | `0x18003de90` |
| `CCMSSEfxMod` | `0x180188a98`（COL `0x1801c0ca8`） | `0x1800414b0` | `0x180040c20` |

### 23.3 调用点（E44）

```
CBassManagementEfxMod::+0x40 = FUN_18003de90
  18003dee9 MOV RCX,[RSI+0x168]   arg1 = SmallSpeakers mixer
  18003df52 CALL 0x180088ff0

CCMSSEfxMod::+0x40 = FUN_180040c20
  180040fa9 MOV RCX,[RDI+0x1d0]   arg1 = LFE mixer
  180041019 CALL 0x180088e60
```

`FUN_180088e60` 的实参：`RCX` mixer、`RDX` `[[this+0x188]+0x20]`（声道 3 缓冲）、
`R8` 块描述符、`R9` `[this+0x198]`（dst）、`[rsp+0x20]` `[this+0x28]`（inMask）、
`[rsp+0x28]` `[this+0x40]`（outMask）、`[rsp+0x30]` `[this+0x140]`（mixMask）、
`[rsp+0x38]` `([this+0x144] == 1)`（bypass）。

`xref 0x180088e60` 共 4 条：3 条 `.pdata`，唯一 `CALL` 在 `0x180041019`。
`xref 0x180088ff0` 的唯一 `CALL` 在 `0x18003df52`。
`xref 0x1800891f0` 只有 3 条 `.pdata` 记录，没有任何 `CALL`；
[推断] 它是异常处理与内联生成的副本，不是第三实现（可达性未独立验证）。

### 23.4 两个常量（E45，二次校正）

| 地址 | 位模式 | 值 | dB | 使用者 |
|---|---|---|---|---|
| `0x180191738` | `0x3e361887` | 0.17782794 | -15 | `FUN_180088ff0` 的 rest、`FUN_1800891f0` 的 rest、`FUN_180089780` 的非 LFE 声道 |
| `0x18019173c` | `0x3f0ff59a` | 0.56234133 | -5 | `FUN_180088e60` 的单源、`FUN_180089780` 的 LFE |
| `0x180191740` | `0x405660bd` | 3.3496544 | +10.5 | 三条流水线的第 7 步 |

§20.2 的首次“修正”把前两个地址写反。以本节为准：
`FUN_180088ff0` 的交声道净增益是 **-4.5 dB**，
`FUN_180088e60` 的净增益是 **+5.5 dB**。

### 23.5 覆盖矩阵

| 项 | 状态 |
|---|---|
| 两个 mixer 的唯一写入者 | 已解（E42） |
| 两个宿主的 vftable/init/process | 已解（E43） |
| `SmallSpeakers → CBassManagementEfxMod` | 已解（E43/E44） |
| `LFE → CCMSSEfxMod` | 已解（E43/E44） |
| `FUN_180088e60` / `FUN_180088ff0` 的唯一调用点 | 已解（E44） |
| `FUN_1800891f0` 的可达性 | 推断（`.pdata` 副本），未独立验证 |
| 两个增益常量的位值与 dB | 已解（E45） |
| CMSS 链下游处理 | 未解 |
| 实机听感/扫频 | 未执行（无设备） |

详细依据：[g6-bass-dialog-vtable.md](g6-bass-dialog-vtable.md) §13.12。
