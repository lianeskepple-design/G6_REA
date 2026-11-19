# Sound BlasterX G6 逆向与 Rust 复刻

把 Creative Sound BlasterX G6 / Sound Blaster Command 的 Windows 音频处理链拆开，并用 C + Rust 重建一套不依赖 G6 硬件的离线 / 实时客户端。

目标文件：`KSUSBAPO64.dll` 1.2.109.0  
SHA-256：`f7c649544e8d738df4fb814609f2adc48fbe95091a1ae11ac842910020037156`

> 当前状态：主机 APO 的 EQ、Crystalizer、CMSS3D Surround、Smart Volume 已还原并有测试；Bass / Dialog+ 是参考近似；第 5/6 项（Scout 实机十段、硬件回环）因没有实体 G6 暂停。

## 这是什么

这个仓库做三件事：

1. **逆向** `KSUSBAPO64.dll`、`KSUSBSPI64.dll` 与 Sound Blaster Command 的参数/效果结构，记录 vtable、处理函数、系数与数据流。
2. **复刻** 已确认的 DSP：EQ、Crystalizer、CMSS3D Surround、Smart Volume，以及 Bass / Dialog+ 的参考实现。
3. **做成能用的客户端**：离线 WAV 处理 `sbc-dsp`、实时 WASAPI 桌面客户端 `sbc-live`、纯 EQ 工具 `sbc-eq`。

项目不包含 Creative 原厂 DLL、抓包、构建产物或生成的 WAV；这些都在 `.gitignore` 里。

## 已实现

| 模块 | 状态 | 说明 |
|---|---|---|
| **EQ** | 已按 APO 原厂系数实现 | 10 段（低架 + 8 峰值 + 高架），耳机/扬声器预设，preamp/headroom；Rust ↔ C 7/7 位级一致 |
| **Crystalizer** | 渲染链已还原 | 3 阶分析滤波、动态系数、128 样本块、256 点 Hann 两缓冲 OLA、`+0x1F8` dry 叠加、128 样本延迟、`+0xF18` 输出级（默认直通） |
| **Surround** | CMSS3D 矩阵层 | 5 声道矩阵、17.5° 常量、mask/strength、过渡/稳态斜坡；仅 6 声道 5.1 |
| **Smart Volume** | 全带 AGC | 电平、mix、攻击/释放；不是多频段 AGC+DRC |
| **Bass** | 参考近似 | Missing Fundamental：低通 → x²/x³ → 高通 → 平滑；`apo-legacy` 为历史实验模式 |
| **Dialog+** | 参考近似 | 2–4 kHz 带通 + 包络/慢噪声底 + `snr/(snr+1)` 门 |

处理链顺序：

```text
EQ -> Crystalizer -> Bass -> Dialog+ -> Surround(仅 6ch) -> Smart Volume
```

> **Surround 说明**：矩阵公式、常量和 13 个 mask 分支是从 `KSUSBAPO64.dll` 反编译出来的，C 实现与独立 Python 参考逐位对拍通过（`verify_surround.cmd`）。但 mask 的声道语义（FL/FR/FC/LFE/BL/BR）有推断成分，且没有 HRTF FIR、ITD/IID、耳机专用路由；它不是算法工程师 PDF 里的 HRTF 渲染器。详见 [g6-algorithm-review.md §2](creative-sbcommand/reference/g6-algorithm-review.md)。

## 仓库结构

```text
creative-sbcommand/
  app/          Rust：EQ 内核 + sbc-live / sbc-dsp / sbc-eq
  reference/    C DSP 实现、头文件、逆向记录、验证脚本
  tools/        Ghidra/IDA 辅助、EQ 参数提取、Scout 扫频/USBPcap、验证工具
README.md       本页
```

`dist/`、`app/target/`、`capture/`、`investigation.cany/`、`tools/audio-env/` 等生成物/本地工具链不入库。

## 快速开始

### 环境

- Windows 10/11 x64
- Rust stable，MSVC toolchain
- Visual Studio 2022 Build Tools（C++ 工作负载）
- PowerShell 或 cmd

### 构建

```powershell
cd creative-sbcommand/app
cargo build --release
```

`build.rs` 会生成 `target/g6dsp-compile.bat`，调用 `vcvars64.bat`，用 MSVC 编译 `reference/` 下的 C 源并打包成 `g6dsp.lib`。如果 vcvars 不在默认路径：

```powershell
$env:G6_VCVARS = "D:\VS\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
cargo build --release
```

静态 CRT（免装 VC++ 运行库）：

```powershell
$env:RUSTFLAGS = "-C target-feature=+crt-static"
$env:G6_STATIC_CRT = "1"
cargo build --release --target x86_64-pc-windows-msvc
```

### 运行

```powershell
# 实时客户端：选设备、Start
sbc-live.exe

# 列设备 / 无界面自测
sbc-live.exe --list-devices
sbc-live.exe --selftest 3 --input "麦克风名称" --output "输出设备"

# 离线 WAV
sbc-dsp.exe --process-wav in.wav out.wav --crystalizer 50 --svm-level -20
sbc-dsp.exe --process-wav in.wav voice.wav --bass 80 --bass-cutoff 120 --dialog 60
sbc-dsp.exe --process-wav six.wav surround.wav --surround-mask 0x3f --surround-strength 50

# 仅 EQ
sbc-eq.exe --list-eq-presets
sbc-eq.exe --process-wav in.wav out.wav --eq-preset Acoustic --output-mode headphones
```

详细参数见 [sbc-dsp CLI](creative-sbcommand/reference/sbc-dsp-cli.md) 与 [sbc-live](creative-sbcommand/reference/sbc-live.md)。

## 验证

```powershell
cd creative-sbcommand/app
cargo test          # 31 passed (22 + 6 + 3)

cd ../tools/apo-eq-coeff
verify_crystalizer.cmd   # PASS: delay_ok 1, drymix_ok 1, drymix_delta 0
verify_surround.cmd      # PASS
verify_svm.cmd           # PASS
```

已实测：

- 2ch 直通逐位一致；Crystalizer / SVM / 6ch Surround / EQ 预设输出全有限。
- 实时 UU 虚拟对 30 秒：buffer 48 ms，0 underrun / 0 overrun。
- USB 麦克风 → 蓝牙耳机：buffer 170 ms，0 / 0。
- 免安装包静态 CRT，`dumpbin -dependents` 只剩系统 DLL。

## 逆向记录

| 文档 | 内容 |
|---|---|
| [offline-progress.md](creative-sbcommand/reference/offline-progress.md) | 总进度、测试证据、边界 |
| [g6-bass-dialog.md](creative-sbcommand/reference/g6-bass-dialog.md) | Bass / Dialog+ 参考实现与逆向状态 |
| [g6-bass-dialog-vtable.md](creative-sbcommand/reference/g6-bass-dialog-vtable.md) | Bass `CBassManagementEfxMod` / `CBassMgmtEfxMod`、Dialog SPI vtable |
| [g6-algorithm-review.md](creative-sbcommand/reference/g6-algorithm-review.md) | 与算法工程师 PDF 的逐项对照 |
| [sbc-live.md](creative-sbcommand/reference/sbc-live.md) | 实时客户端、WASAPI、打包 |
| [sbc-dsp-cli.md](creative-sbcommand/reference/sbc-dsp-cli.md) | 离线 CLI 参数与用例 |
| [hardware-runbook.md](creative-sbcommand/tools/scout-measure/hardware-runbook.md) | G6 实机 Scout / 回环流程 |

## 边界

- **没有实体 G6**：Scout 实机十段、USB id↔频段实测、硬件回环暂停。
- **Bass / Dialog+ 不是逐位还原**：SBX Bass 的 Missing Fundamental 与 Dialog+ 的实际 DSP 不在主机 APO/SPI 里；当前是 PDF 方向的参考近似。
- **Surround 不是 HRTF**：CMSS3D 矩阵层已逆向并对拍；mask 声道语义有推断成分，没有 HRTF FIR、ITD/IID 或耳机专用路由。
- **SVM 是全带简化版**：不是多频段 AGC+DRC+limiter。
- **实时客户端只做立体声**：Surround 未放进 `sbc-live`。
- **Crystalizer `+0xF18` 默认直通**：系数源仍未找到。

## 说明

本项目用于互操作、音频研究与个人复刻。Sound Blaster、Sound BlasterX、G6 是 Creative Technology Ltd. 的商标；仓库不包含 Creative 的二进制文件或固件。
