# 离线 WAV EQ 实现与验证记录

日期：2026-11-12。实现及验证均在本地进行；不访问声卡、不回环、不录音、不加载或调用厂商 DLL。

## 1. 实现范围

- 实际可执行目标：`app/src/bin/sbc-eq.rs`，命令名 `sbc-eq`；依赖 `hound = 3.5.1`。DSP 与预设解析位于库目标 `app/src/eq_core.rs`（crate `aurora_audio_command`），桌面 Equalizer 页与 CLI 共用。
- 编译期以 `include_str!("../../reference/g6-eq-presets.json")`（来自 `eq_core`）嵌入 G6 EQ JSON，实际列出 34 个预设。运行不需要再读取该 JSON 文件。
- `--list-eq-presets` 列出全部预设。
- `--process-wav input.wav output.wav --eq-preset NAME --output-mode headphones|speakers [--headroom-db N]`。
- WAV 输入支持 PCM 16/24/32 bit 与 IEEE float32，输出始终 IEEE float32 WAV；保持采样率、声道数、帧数。
- 两个输出声道分别拥有独立的 10 段滤波器状态。拓扑按 `reference/apo-effects-structure.md`：第 1 段低架、第 2–9 段峰值（`alpha = tan(w0/(2·ratio))`，带宽由相邻频点比推导）、第 10 段高架。**峰值段带宽公式有 APO 指令级证据；架式段用 RBJ 斜率 S=1 近似，APO 架式内部公式尚未完全还原。差分方程形式（DF1/DF2T）也未确认。**
- 使用 JSON 中对应 Speaker/Headphone 的频率、增益及 preamp。`--headroom-db N` 是**带符号的附加增益**，以 dB 叠加在原 preset preamp 上：**负数为衰减（如 -6），正数为加增益（如 +6）**。它**不是自动防削波**，也不启用限幅；正的 headroom 可以让 float32 输出样本超过 1.0。
- 对非法预设、NaN/Inf、采样率为零或无法支持活跃高频段的采样率、已有输出、输入输出路径相同返回非零状态。完成 WAV 解码、预设/采样率验证和滤波处理后才以 `create_new` 创建输出；写入失败会尝试删除本次创建的半成品，若删除也失败则在错误中附带残留路径。**创建任何输出前先检查 `10^(dB/20)` 线性增益有限且不超过 f32 可表示范围；f64 结果转 f32 后再次逐样本检查有限**，因此 +1000 dB 这类 f64 有限但 f32 溢出的参数无论输入是否静音都会非零失败且不写输出。
- 定位为“按原厂频率与增益数值做软件 EQ”。不等同原厂 DSP，也不复现 Scout、SBX、Crystalizer、Bass、Dialog+、Smart Volume 或混响。

## 2. 完整实跑命令与结果

工作目录：`D:\工程\逆向工程\creative-sbcommand`。Rust 工具链由 `C:\Users\Administrator\.cargo\bin\cargo.exe` 调用，测试 Python 为 `tools/audio-env/Scripts/python.exe`。

```text
cd creative-sbcommand/app
C:/Users/Administrator/.cargo/bin/cargo.exe build
结果：成功，Finished dev profile（包含桌面 app 与 sbc-eq）。

C:/Users/Administrator/.cargo/bin/cargo.exe test
结果：成功；19 passed / 0 failed（eq_core 13、桌面 app 6、sbc-eq 与 doc-test 0）。
用例（eq_core）：34 个预设可解析且按原厂 order 排序、名称唯一；Flat 增益 biquad 为恒等；峰值段中心达到标称增益且 DC/Nyquist 为单位增益；低架/高架在各自端点达到标称增益；Speaker/Headphone 两套曲线保留差异；Nyquist 高频段拒绝；带宽比 0/NaN/负数拒绝；NaN/±inf/超大 dB 线性增益拒绝；f64→f32 转换溢出拒绝；单段 1 kHz +6 dB 的整链响应命中 6 dB；preamp/headroom 计入响应；34×2 条原厂曲线响应全程有限。
用例（桌面）：`default_eq_output` 生成 `<stem>_eq.wav`；`apply_preset` 按输出模式取原厂曲线、Personal/自定义可用、未知预设不改曲线；渲染动作输出 float32 WAV 且拒绝覆盖。

../tools/audio-env/Scripts/python.exe ../tools/verify-offline-eq.py --exe target/debug/sbc-eq.exe --presets ../reference/g6-eq-presets.json
结果：E2E PASS。覆盖 PCM16 / PCM24 / PCM32 / float32 输入，输出 float32；PCM16/PCM32 输出样本值与解码输入比对，钉住整数解码缩放；48 kHz 立体声输入保持采样率、声道数和帧数；左声道单独激励时右声道逐样本恰为 0（真实声道隔离）；Flat Speaker 与输入差异 < 2e-7；Speaker 与 Headphone 输出不同；-6 dB 衰减为 0.501x 且与理论一致；+6 dB 使 0.8 样本输出到约 1.596（有限且大于 1.0），证明正 headroom 不自动限幅；31 Hz Headphone 级联实测 +2.389 dB，与低架+峰值+高架级联的理论复数响应差 < 0.15 dB；+1000 dB（含静音输入）、1e10 dB、-inf、NaN、+inf、无效预设、已有目标、硬链接目标和同路径均非零失败，不创建/不覆盖输出；相同输入重复运行输出逐字节一致。脚本使用 `tools/.offline-eq-test/run-*`，连续运行两次均通过；本次运行的临时目录结束后已清理，存在历史遗留 `run-*` 目录时重跑仍通过。

../tools/audio-env/Scripts/python.exe ../tools/verify-offline-eq-independent.py
结果：23/23 通过，更新 ../reference/offline-independent-results.json。覆盖 SciPy SOS 脉冲对比（最大误差 2.27e-09）、左侧单独激励/右侧为 0、Flat 恒等、-6 dB 衰减、+6 dB 非限幅、硬链接/已有输出/同路径保护、+1000 dB（含静音输入）与 1e10 dB 及 NaN/±inf 拒绝、四种 WAV 编码的内容比对、FPS 双模式差异、重复运行逐字节一致、34 个预设 ID（显式计数）。

./target/debug/sbc-eq.exe --list-eq-presets
结果：退出码 0，stdout 恰好 34 行；包括 FootstepEnhancer、Flat 等预设。
```

复现脚本：`tools/verify-offline-eq.py` 与 `tools/verify-offline-eq-independent.py`。两者都在项目内 `tools/.offline-eq-test` 下创建唯一的临时运行目录，并在结束时自动清理，不使用系统临时目录；被中断运行遗留的 `run-*` 目录不会使下一次重跑失败。Rust 构建产物保留在现有 `app/target` 构建目录。

## 3. 设计选择与限制

- 峰值段带宽由 `f[i+1]/f[i]` 决定；首末为架式段，其中心频率处只有约一半的标称增益（架式固有特性）。相邻滤波段的频响会相互叠加；实测频点增益不必恰好等于单个 band 的标称增益。
- 处理内部使用 f64 biquad 运算，最后写为 float32；f64 结果和 f32 转换结果都检查有限性。没有削波/限幅；`--headroom-db` 只是带符号附加增益，负数为衰减、正数为加增益，正数**不提供自动防削波**，调用方需自行控制电平。
- 此实现只覆盖本任务要求的 WAV 和 EQ 功能，没有声卡操作，也没有原厂 DSP、Scout 或其他音效效果的等价性声明。

## 4. 桌面集成（2026-11-13）

- `app/src/eq_core.rs` 成为唯一 DSP 实现；`app/Cargo.toml` 增加库目标 `aurora_audio_command`，CLI 与 GUI 都链接它。GUI 未直接依赖 `hound` 之外的音频设备。
- Equalizer 页预置下拉改为 34 条 G6 原厂预置（`state::eq_presets()`，LazyLock 解析内嵌 JSON），按 `playback.output` 选择 Speaker/Headphone 曲线；旧的 15 条手写 AE5Plus 曲线已删除。
- 曲线图改为同一 `Chain::response_db` 在 31 Hz–16 kHz 对数轴的 160 点采样，含 preamp；节点 x 坐标与点击命中都改用同一对数映射。填充改为逐对采样点的凸四边形，避免单条凹多边形在 egui 里产生的三角化伪影。
- “Render WAV…” 对话框把当前 `bands`（f32→f64）与 `preamp` 组成本轮曲线后调用 `eq_core::render_wav`；输出路径为空时默认 `<stem>_eq.wav`，已存在则报错，不覆盖。
- 截图验证（`--screenshot` + `AURORA_PAGE=equalizer`，另有仅截图模式生效的 `AURORA_EQ_PRESET` / `AURORA_OUTPUT` / `AURORA_EQ_RENDER` 覆盖项）：Flat、Fortnite(Speaker) 与 Render 对话框三帧，存于 `app/checks/round-4/`；Fortnite 的曲线在 62 Hz、2 kHz、8 kHz 处凸起，125/250/4k 处下凹，与频段增益及滤波器响应一致。
- 未接入：实时音频输入/输出、硬件设备通信。桌面只显示计算出的曲线并做离线文件渲染。
