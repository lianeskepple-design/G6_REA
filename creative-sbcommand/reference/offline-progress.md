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
- 输出级：`+0xF18` 固定二阶已接入（默认直通，`g6_crystalizer_set_output_filter` 可装实测系数）；`0x180195BD0` 的 256 点 Hann 窗与 `0x180195FD0` 的 128 点增益斜坡 `1/(1-0.5·sin²(πn/128))` 已实现，探针校验窗口/斜坡值。
- 对照：两路分析系数、16 点脉冲响应、动态系数与独立 float32 重算逐位一致；8192 帧立体声链输出全部有限；0.05 → 0.6 阶跃后第一/第二活动量约 0.37 / 0.29，强度 0 与 100 的输出最大差约 5.2e-3，证明检测器已驱动动态系数与逐声道滤波。
- 两缓冲：`0x18008E920` 每声道取 `[上一块 dry, 当前块]` 共 256 点，乘 Hann → 一阶 → 二阶 → 再乘 Hann；`+0x1F8`（`FUN_180146190`）把 `+0x600` 第二缓冲的上一块 dry 以权重表 `0x180199E80 = {1,1,1,1}` 加到 scratch，随后两缓冲都写入当前 dry；输出取前 128 点乘 ramp，再经 `+0xF18`。首块为零，存在 128 样本延迟。
- 仍未展开：`+0xF18` 系数初始化位置（默认直通，可设为实测值）。

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
- Crystalizer：`+0xF18` 固定二阶输出级接入（默认直通，可设实测系数）；`0x180195BD0` 的 256 点 Hann 窗与 `0x180195FD0` 的 128 点增益斜坡 `1/(1-0.5·sin²(πn/128))` 实现并对照；两缓冲重叠相加与 `+0x1F8` dry 叠加（权重全 1）、128 样本延迟已按 Ghidra 反编译补齐；探针新增窗口/斜坡、输出级、延迟与 dry 叠加检查。
- 仍未展开：`+0xF18` 系数初始化位置；Scout 实机取数需 G6 接入。

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
- **2026-11-19 修正**：继续反编译确认 `FUN_1800cc560` 的 10-tap FIR 与 `FUN_1800cb5e0` 的三次多项式作用在**增益包络**上，不是音频；`CBassManagementEfxMod` 是软拐点限幅/动态链（包络→20log10→限幅→FIR 平滑→增益映射→乘回音频）。`FUN_180089780` 是 bass management 分频/路由核心：每声道一阶高通（`H(z)=c(z⁻¹-1)/(1+a2z⁻¹)`，`c=-2/(k+2)`、`a2=(k-2)/(k+2)`、`k=3π*cutoff/fs`），卫星声道去低频；同时把**原始** 6 声道以 -15 dB 汇总到 LFE/sub。不是低音提升，也不是 Missing Fundamental 谐波。`mode=1` 因此标记为 legacy 误读，仅保留历史对比。
- Dialog 补充：`CVocalSuppressorEfxMod` vftable `0x18018de98`、init `0x180077b20`、process `0x180077860` 已解；它是 4 段可交叉淡化滤波器组（属性 GUID `{1ee40950-a46c-45eb-a50c-f1fd995c0166}`，index 0..7），不是 Dialog+ 的 2–4 kHz 对白增强，未找到 `THXDialogueAdvantage` 的实际 DSP。32/64 位 APO 的 EfxMod 类名列表一致（118 个，均无 Dialog 类）；`KSUSBDGFX64.dll`/`KSUSBWrap64.dll` 只是 APO 包装器（IAudioSystemEffects / DaisyWrapEFX），无 DSP。
- THX Bass 补充：`CBassMgmtEfxMod` vftable `0x18018cb38` 是另一条低音管理链：`FUN_180067890` 解析 `bass` 配置文件的 `crossover`/`boostlevel`/`virtuallevel`，`FUN_1800a9670` 做双二阶交叉滤波 + 128 帧参数淡化；它做分频/路由，不是 Missing Fundamental 谐波合成。
- 文档：`reference/g6-bass-dialog.md`、`reference/g6-bass-dialog-vtable.md`；PDF 对照更新 `reference/g6-algorithm-review.md`。

| 检查 | 结果 |
|---|---|
| `sbc-dsp --bass 100` | 输出有限，峰值 0.524，谐波生成符合预期 |
| `sbc-dsp --bass 100 --bass-mode apo` | 输出有限，峰值 0.464，180/300 Hz 谐波 |
| `sbc-dsp --dialog 100` | 输出有限，峰值 0.674，3 kHz 提升 |
| `sbc-live --selftest` + Bass/Dialog | 5 秒 0 underrun / 0 overrun |




