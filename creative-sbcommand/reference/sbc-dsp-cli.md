# sbc-dsp：离线 WAV DSP 客户端

2026-11-19。把已还原的 EQ、Crystalizer、Surround、Smart Volume，以及
Bass/Dialog+ 参考近似串成一个 Rust 命令行客户端，读 WAV → 处理 → 写
float32 WAV。不依赖 G6，也不做实时播放。

## 处理顺序

```text
EQ (Rust, APO 精确系数)
  -> Crystalizer (C)
  -> Bass (C, Missing Fundamental 参考)
  -> Dialog+ (C, 2-4 kHz + 轻量门参考)
  -> Surround (C, 仅 6 声道)
  -> Smart Volume (C)
```

- EQ 使用 `aurora_audio_command::eq_core`，与桌面 EQ 页/`sbc-eq` 同一内核。
- C 侧通过 `reference/g6_chain_capi.h` 的不透明句柄调用，Rust 不复制 C 结构体布局。
- Bass/Dialog+ 是按调研 PDF 算法类别做的参考近似，不是 G6 硬件逐位实现；
  详见 [g6-bass-dialog.md](g6-bass-dialog.md)。
- 内部按 128 帧分块，Surround 的过渡斜坡与 DLL 的块行为一致。
- `--mode direct|sbx|scout` 与 `--sbx-profile NAME` 实现产品的模式语义
  （见 [sbx-modes.md](sbx-modes.md)）：Direct/Scout 全旁路，SBX 按 profile 的
  Enable+Level 装载；profile 的 Surround 只对 6 声道输入生效。

## 构建

```powershell
cd app
cargo build --bin sbc-dsp
```

Windows 下 `build.rs` 会生成 `target/g6dsp-compile.bat`，调用
`vcvars64.bat` 后用

```text
cl /nologo /TC /std:c11 /W4 /fp:strict /I ..\reference ...
lib /nologo /OUT:g6dsp.lib ...
```

编译并打包成静态库，再交给 Rust 链接。`vcvars64.bat` 默认按 VS 2022
BuildTools 路径查找；其他安装位置用 `G6_VCVARS` 指向实际文件：

```powershell
$env:G6_VCVARS = "D:\VS\BuildTools\VC\Auxiliary\Build\vcvars64.bat"
cargo build --bin sbc-dsp
```

非 Windows 目标退回 `cc` 构建。工程路径含中文时，编译器对象输出放在
`app/target/g6dsp-cc-<profile>`，避免把非 ASCII 路径放进 `cl` 的 `-Fo`。

## 用法

```text
sbc-dsp --process-wav input.wav output.wav [options]

  --eq-preset NAME         应用工厂 EQ 预设
  --output-mode MODE       headphones|speakers（与 --eq-preset 同时使用）
  --headroom-db N          预设 preamp 上的有符号 dB（默认 0）
  --crystalizer N          Crystalizer 强度 0..100
  --bass N                 Missing Fundamental Bass 强度 0..100
  --bass-cutoff HZ         Bass 截止 40..400 Hz（默认 120）
  --bass-mode MODE         reference|apo-legacy（默认 reference）
  --dialog N               Dialog+ 强度 0..100
  --surround-mask MASK     CMSS3D 掩码，十进制或 0x..；要求 6 声道
  --surround-strength N    Surround 强度 0..100（与 mask 同时使用）
  --svm-level DB           Smart Volume 电平 dB
  --svm-mix MIX            Smart Volume 混合 0..1（默认 1.0）
```

输入输出不能是同一文件，输出必须不存在；输出为 float32 WAV。

示例：

```powershell
# 只做直通/格式转换
sbc-dsp --process-wav in.wav bypass.wav

# Crystalizer + Smart Volume
sbc-dsp --process-wav in.wav chain.wav `
  --crystalizer 50 --svm-level -20

# Bass + Dialog+
sbc-dsp --process-wav in.wav voice.wav `
  --bass 80 --bass-cutoff 120 --dialog 60

# 5.1 WAV：FL FR FC LFE BL BR
sbc-dsp --process-wav six.wav surround.wav `
  --surround-mask 0x3f --surround-strength 50

# EQ 预设 + 两个 C 效果
sbc-dsp --process-wav in.wav out.wav `
  --eq-preset Acoustic --output-mode headphones `
  --crystalizer 25 --svm-level -30
```

## 已验证

用 48 kHz 合成 WAV 跑通：

| 用例 | 结果 |
|---|---|
| 2ch 直通 | 输出与输入逐位一致 |
| 2ch Crystalizer 50 + SVM -20 | 输出全有限，RMS 0.0707 → 0.0506，最大差 0.316 |
| 6ch Surround mask 0x3f / 50 | 输出全有限；FL/FR/BL/BR 最大差约 0.047–0.050，FC/LFE 不变 |
| EQ 预设 + Crystalizer + SVM | 输出全有限，最大差 0.255 |
| Bass 100，60 Hz 输入 | 120 Hz -26.4 dB、180 Hz -39.9 dB；峰值 0.524 |
| Bass 100 + `--bass-mode apo` | 180 Hz -20.8 dB、300 Hz -30.5 dB、120 Hz -45.8 dB；峰值 0.464（legacy 实验，非 APO 逐位） |
| Dialog+ 100，1k+3k 输入 | 3 kHz 相对 1 kHz +5.8 dB；峰值 0.674 |
| stereo 上启用 Surround | 退出码 1，提示需要 6 声道，未创建输出文件 |
| `cargo test` | 31 passed |

## 当前边界

- **离线文件处理，不是实时客户端**。没有 WASAPI/ASIO 音频线程。
- **Surround 只接受 6 声道 5.1 顺序**；不做立体声上混。
- Bass/Dialog+ 是按 PDF 算法类别做的参考近似，不是 G6 硬件/APO 的逐位实现。
- `--bass-mode apo` 是 2026-11-19 之前的误读产物（FIR/整形实际作用于增益
  包络），仅保留作历史对比；请用默认 `reference`。
- Crystalizer 只处理前 3 个声道（与 DLL 一致）。
- SVM 的两抽头交叉淡化与各模式曲线仍未细化；当前是统一增益律。
- Crystalizer 没有 `+0xF18` 输出级：`.text` 中该位移 0 次命中，ramp 混合就是最后一级。
- Smart Volume 已按 `CPlugAGCCompressor` 逐行移植（20 ms 前瞻、两抽头交叉淡化、
  三级平滑、`10^1.1` 补偿增益）；`--svm-level` 的 `(level_db-12)/10` 是 legacy 映射，
  DLL 内没有电平旋钮，新增 `--svm-delay-ms`。见 [g6-svm-exact-port.md](g6-svm-exact-port.md)。
- 模式与 profile：`--mode` / `--sbx-profile`，Direct 输出与输入逐位一致；
  profile 的 SVM 电平用 `-50 + 0.3*level` 的标注近似映射到 dB。见 [sbx-modes.md](sbx-modes.md)。
- 未在 G6 实机上做回环验证。

## 文件

- `app/src/bin/sbc-dsp.rs`：CLI、参数解析、Rust EQ、FFI 句柄。
- `app/build.rs`：MSVC 批处理/静态库构建，或非 Windows 的 `cc` 构建。
- `reference/g6_chain_capi.h` / `.c`：C 包装层。
- `app/src/eq_core.rs`：新增 `load_wav_f32` / `write_wav_f32`。
- `app/src/bin/sbc-dsp.rs` 内 3 个测试：stereo 链路、6ch Surround、Surround 声道校验。
