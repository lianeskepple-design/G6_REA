# sbc-live：实时 WASAPI 桌面客户端

2026-11-19。Rust + eframe 的实时监听客户端：麦克风 → EQ → Crystalizer →
Smart Volume → 耳机/扬声器。用 `cpal` 的 WASAPI 后端，C DSP 链与 `sbc-dsp`
共用同一套静态库。

## 快速开始

```powershell
cd app
cargo run --release --bin sbc-live
```

界面里选择输入（如 `麦克风 (USB PnP Sound Device)` 或
`麦克风阵列 (UU远程虚拟音频设备)`）和输出（耳机/扬声器），点 **Start**。
建议戴耳机，避免麦克风拾取扬声器产生回授。Crystalizer、SVM、输入增益可
运行中调；EQ 预设在 Start 时应用。

## 架构

```text
WASAPI capture callback
  -> 1/2 声道转立体声 + 输入增益
  -> Rust EQ（启动时按预设构建）
  -> C chain: Crystalizer -> SVM（Surround 不启用）
  -> SPSC ring (rtrb)
  -> WASAPI render callback -> 设备
```

- 采集和渲染是两条 cpal 流，中间用 `rtrb` 单生产者/单消费者环形缓冲。
- 采集回调只做定点处理：无堆分配、无锁；参数通过 `AtomicBool`/`AtomicU32` 读取。
- `ChainHandle` 只被移动进采集回调，`unsafe impl Send` 仅用于满足 cpal 的闭包约束。
- 内部按 128 帧处理，Crystalizer 自带 128 样本延迟；启动前预缓冲 60 ms，
  界面显示当前 buffer ms。
- 设备统一选 48/44.1/96/32 kHz 中双方都支持的 F32 配置；输入 1 或 2 声道，输出要求 2 声道。

## 构建

```powershell
cd app
cargo build --bin sbc-live
# 或
cargo build --release --bin sbc-live
```

`sbc-live` 复用 `sbc-dsp` 的 `build.rs`（MSVC 批处理 + `g6dsp.lib`）。

## 使用

### 图形界面

```powershell
app\target\debug\sbc-live.exe
```

界面提供：

- 输入/输出设备下拉框、刷新、Start/Stop。
- Input gain（-12..+36 dB）。
- Crystalizer 开关 + 强度 0..100（运行中实时生效）。
- Bass 开关 + 强度 + 截止频率（运行中实时生效；参考近似）。
- Dialog+ 开关 + 强度（运行中实时生效；参考近似）。
- Smart Volume 开关 + Level/Mix（运行中实时生效）。
- EQ 预设 + Headphones/Speakers；EQ 在 Start 时构建，运行中不可改。
- 输入/输出 RMS/峰值表、underrun/overrun 计数。
- 启动时加载 `C:\Windows\Fonts\Deng.ttf`（回退 SimHei / msyh.ttc），
  解决 egui 默认字体把中文设备名显示成方框的问题。
- 建议用耳机，避免麦克风拾取扬声器造成回授。

### 设备枚举

```powershell
app\target\debug\sbc-live.exe --list-devices
```

本机输出示例（WASAPI）：

```text
IN   0: 麦克风阵列 (UU远程虚拟音频设备) [48000 Hz, 2 ch, F32]
IN   1: 耳机 (MILTON A.N.C.) [16000 Hz, 1 ch, F32]
IN   2: 麦克风 (USB PnP Sound Device) [48000 Hz, 2 ch, F32]
OUT  0: 扬声器 (USB PnP Sound Device) [48000 Hz, 2 ch, F32]
OUT  1: 耳机 (MILTON A.N.C.) [48000 Hz, 2 ch, F32]
OUT  2: 扬声器 (UU远程虚拟音频设备) [48000 Hz, 2 ch, F32]
```

### 无界面自测

```powershell
app\target\debug\sbc-live.exe --selftest 2 `
  --input "麦克风阵列 (UU远程虚拟音频设备)" `
  --output "扬声器 (UU远程虚拟音频设备)" `
  --crystalizer 50 --svm-level -20 `
  --eq-preset Acoustic --eq-mode headphones
```

30 秒实测（UU 虚拟输入/输出，无效果）：输入 RMS 0.0018、峰值 0.0033；
输出 RMS 0.0017、峰值 0.0034；buffer 48 ms；underrun 0、overrun 0。

10 秒实测（UU 虚拟对 + EQ Acoustic + Crystalizer 50 + SVM -20）：输入 RMS
0.0001、峰值 0.0003；输出 RMS 0.0001、峰值 0.0002；buffer 48 ms；
underrun 0、overrun 0。

10 秒实测（USB PnP 麦克风 → MILTON 蓝牙耳机）：buffer 170 ms；
underrun 0、overrun 0。说明采集、Rust EQ、C DSP、环形缓冲、渲染整条链
在 WASAPI 上稳定工作。

启动前会先开采集流、等 60 ms 填环形缓冲，再开渲染流；这消除了原先
启动期的 2–5 次 underrun。

## 免安装包

```text
dist/sbc-live-1.0.0-win64.zip
SHA-256 fe3cff79442e5d1c328ff3942b304e8c285022d929d9dc635c9d70a56b29c76b
```

内容：`sbc-live.exe`、`sbc-dsp.exe`、`sbc-eq.exe`、`README.txt`、
`run-sbc-live.cmd`、`SHA256SUMS.txt`。本版 `sbc-dsp` 已带
`--bass-mode apo`（实验链），`sbc-live` 未变。

- 静态 CRT 构建（`RUSTFLAGS=-C target-feature=+crt-static` +
  C 侧 `/MT`），`dumpbin -dependents` 只列出系统 DLL，不需要
  `vcruntime140.dll` / `msvcp140.dll`，目标机不用装 Rust 或 VC++ 运行库。
- 解压后直接运行 `sbc-live.exe`；`--list-devices`、`--selftest` 仍可用。
- 包内校验：解压后按 `SHA256SUMS.txt` 对 5 个文件逐一比对通过。

## 边界

- 只做立体声麦克风实时监听；Surround 需要 6 声道，未放进这个客户端。
- 使用 WASAPI 共享模式和设备默认缓冲；没有 ASIO/独占模式。
- 启动前预缓冲 60 ms；界面显示当前 buffer ms，实测 UU 虚拟对 30 秒
  0 underrun / 0 overrun。
- Crystalizer 固定 128 样本延迟，加上设备缓冲，总延迟约几十毫秒。
- EQ 预设只在 Start 时应用；Crystalizer/SVM/输入增益可运行中调。
- SVM 没有模式曲线（DLL 只导出 on/off、延迟、起音/释放）；两抽头交叉淡化与三级平滑已按
  [g6-svm-exact-port.md](g6-svm-exact-port.md) 实现，默认前瞻 20 ms。
- Crystalizer 没有 `+0xF18` 输出级（`.text` 中该位移 0 次命中），已从代码中移除。
- 未做 G6 实机验证；当前验证使用 Windows 默认/WASAPI 设备与 UU 虚拟设备。

## 文件

- `app/src/bin/sbc-live.rs`：GUI、cpal 流、环形缓冲、参数原子量。
- `app/build.rs`：C DSP 静态库构建（与 `sbc-dsp` 共用）。
- `reference/g6_chain_capi.h/.c`：C 包装层。
- `reference/sbc-dsp-cli.md`：离线 WAV 版本。
