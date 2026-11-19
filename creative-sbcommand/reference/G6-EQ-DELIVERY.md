# Sound Blaster G6 EQ 参数库交付说明

## 数据范围

交付数据采自 2026-11-19 本机 Sound Blaster Command 的 G6 安装目录
（%ProgramData%\Creative\SBCommand\Product\G6\Presets\EQ）。共 43 个预设、
86 条模式记录。随附的 g6-eq-presets-current.json 记录每个预设的源文件名、
SHA-256、原厂 ID 和两套参数。历史 34 项快照仍保存在 g6-eq-presets.json。

本交付验证 EQ 参数和 C 接口。原厂 DSP 算法、设备写入、实际频响及文件的
出厂身份需要各自的实机或安装包证据。

## 接入 C 项目

把 g6_eq_presets.c 加进项目的编译源文件，将 g6_eq_presets.h 所在目录
加入头文件搜索路径。表项只读，无动态分配和运行时 JSON 依赖。示例选取
FPS 的耳机设置，查询失败时按应用自己的约定处理。

~~~
#include "g6_eq_presets.h"
#include <stdio.h>

int main(void)
{
    const G6EqPreset *preset = g6_eq_find_preset("FPS");
    if (preset == NULL) {
        return 1;
    }

    const G6EqSettings *eq = &preset->headphone;
    printf("Preamp: %.1f dB\n", eq->preamp_db);
    for (size_t i = 0; i < G6_EQ_BAND_COUNT; ++i) {
        printf("%u Hz: %.1f dB\n", g6_eq_band_hz[i], eq->gain_db[i]);
    }
    return 0;
}
~~~

在 MSVC 开发者终端可用 cl /TC /std:c11 /W4 /WX main.c g6_eq_presets.c 编译示例。
扬声器模式取 preset->speaker。ID 区分大小写，按 JSON 内的 Id 查询，
例如 EscapefromTarkov.json 的 ID 为 EFT。遍历时使用 g6_eq_presets 与
g6_eq_preset_count；界面如需按原厂顺序显示，可读取 order。type 和
speaker_method 原样保留，供调用方参考。

gain_db 的十个下标按 g6_eq_band_hz 排列，依次为 31、62、125、250、500、
1000、2000、4000、8000、16000 Hz。preamp_db 和 gain_db 均为 dB。

## 处理 PCM

将 g6_eq_dsp.c、g6_eq_apo.c 和 g6_eq_presets.c 一起编译，并包含 g6_eq_dsp.h。
在需要显式链接数学库的环境，链接时加 -lm。双精度链采用双精度系数与
状态；APO 精确链采用 float32 系数与 Direct Form II 状态。处理函数均不分配内存。

下面用 FPS 耳机曲线处理交错排列的双声道 float PCM。调用方提供长度为
frame_count × 2 的缓冲区。设计 G6EqChain 一次，然后对每个后续音频块沿用
同一组 channel_state，以保持滤波器连续。

~~~c
#include "g6_eq_dsp.h"

G6EqStatus init_fps_eq(G6EqChain *chain, G6EqChannelState state[2])
{
    const G6EqPreset *preset = g6_eq_find_preset("FPS");
    if (preset == NULL) {
        return G6_EQ_INVALID_ARGUMENT;
    }
    G6EqStatus status = g6_eq_chain_design(chain, &preset->headphone, 48000, -3.0);
    if (status == G6_EQ_OK) {
        g6_eq_reset_channel(&state[0]);
        g6_eq_reset_channel(&state[1]);
    }
    return status;
}

G6EqStatus apply_fps_eq(const G6EqChain *chain, G6EqChannelState state[2],
                        float *pcm, size_t frame_count)
{
    G6EqStatus status = g6_eq_process_interleaved(chain, state, pcm, frame_count, 2);
    if (status == G6_EQ_SAMPLE_RANGE) {
        g6_eq_reset_channel(&state[0]);
        g6_eq_reset_channel(&state[1]);
    }
    return status; /* Discard the block if processing failed. */
}
~~~

- g6_eq_chain_design 将预设前级和有符号附加增益合并。0 dB 的频段会跳过，
  已启用频段必须低于奈奎斯特频率。也可用自定义的十段 G6EqSettings。
- g6_eq_process_sample 提供双精度单声道逐样本接口，需要每个声道独立的
  状态。切换预设或采样率后，重新设计系数并清空旧状态。
- float 处理保留大于 1.0 的有限浮点样本。出现非有限输入或结果无法用
  float 表示时，返回 G6_EQ_SAMPLE_RANGE，当前块的一部分可能已经处理。
- g6_eq_response_db 查询任意频点的理论增益，包含前级。滤波为 1 低架、
  8 峰值和 1 高架；双精度链沿用早期 RBJ 系数，APO 精确系数链见下节。
  WAV 文件读写由调用方负责。

## APO 精确系数链

g6_eq_apo.c/h 是 KSUSBAPO64.dll 三个系数函数（VA 0x180087120 /
0x180087970 / 0x1800887E0）和逐样本内核（0x1801469E0）的指令级移植。
系数和状态均为 float32，内核为 Direct Form II；槽位与差分方程见
g6_eq_apo.h。G6EqApoChain 与 G6EqApoChannelState 的用法与双精度链相同：

~~~c
G6EqApoChain chain;
G6EqApoChannelState state[2] = {0};
G6EqStatus status = g6_eq_apo_chain_design(&chain, &preset->headphone, 48000, -3.0);
if (status == G6_EQ_OK) {
    status = g6_eq_apo_process_chain_interleaved(&chain, state, pcm, frame_count, 2);
}
~~~

峰值段按功率比 10^(dB/10) 换算，架式段按幅度比 10^(dB/20) 换算，
使两类滤波器的平台/中心都命中 UI 的 dB 标签；DLL 的原始线性入口
（g6_eq_apo_*_linear）也保留。峰值段中心、低架 DC、高架 Nyquist 的
标签 dB 检查已纳入 verify-g6-eq-presets.ps1。该链与 Rust 实现
（app/src/apo_eq.rs）在 7 组用例、每组 5 个系数槽位和 256 点脉冲响应上
逐位一致；细节与复现见 reference/apo-eq-exact-port.md。

## 离线频响图

解压后用浏览器打开 [g6-eq-response.html](g6-eq-response.html)。
可切换 43 个预设、扬声器或耳机模式和采样率，也可叠加比较两条曲线。
预设下拉框显示中文备注和英文 ID。中文备注存于参数快照的 label_zh 字段，C 接口按英文 ID 查询。
页面内嵌参数，不需要联网或本地服务器。曲线为项目软件 EQ 的理论频响，
含预设前级增益，实体 G6 的实际频响需另行测量。

## 复跑验收

在交付目录执行以下 PowerShell 命令。需要 PowerShell 7.2+ 和支持 C11 的
MSVC Build Tools。脚本优先使用终端里的 cl.exe，也会尝试自动查找 Build Tools。

~~~powershell
pwsh -NoProfile -File .\verify-g6-eq-presets.ps1
pwsh -NoProfile -File .\verify-g6-eq-presets.ps1 -SourceDirectory "$env:ProgramData\Creative\SBCommand\Product\G6\Presets\EQ"
~~~

第一条在只有交付文件的机器上可用，第二条附加复核 43 个源文件的 SHA-256
及其实际字段。脚本编译并运行 C 程序，核对 43 个 ID、86 条模式记录的元数据、
前级增益、频点和增益，并检查大小写、空字符串、NULL 和未知 ID 的查询行为。
DSP 基础检查覆盖前级增益、浮点不削波、非平直预设有滤波输出及左右声道隔离。
APO 精确链另检查旁路、非平直输出、声道隔离和峰值槽位参考值。
通过时输出 PASS 和交付数据的 SHA-256，验收不符时返回非零退出码。

## 与 34 项历史快照的差异

新增 ID 为 BattleRoyale、Cyberpunk2077、DeathStranding、DoomEternal、
EFT、GenshinImpact、MOBA、RDR2、Rainbow6Siege。历史 34 个 ID 均保留。

下表列出同名预设发生变化的频段，数值为旧值 → 当前值（dB）。

| ID | 模式 | 变化频段 |
|---|---|---|
| CallofDutySeries | Speaker | 125 Hz 0 → 2，1 kHz 0 → 2，2 kHz 0 → 3 |
| CallofDutySeries | Headphone | 31 Hz 0 → 4，125 Hz −3 → −1，250 Hz −3 → −2，500 Hz −2 → 2，1 kHz −2 → 2，4 kHz 0 → −4，16 kHz 0 → 2 |
| FPS | Speaker | 31 Hz 0 → 3，62 Hz 0 → 3，1 kHz 0 → 3，2 kHz 0 → 3，4 kHz 0 → −3 |
| FPS | Headphone | 31 Hz 4 → 2，125 Hz 0 → −2，250 Hz 0 → −2，1 kHz −2 → 2，2 kHz −2 → 2，4 kHz 0 → −5，8 kHz 0 → 2，16 kHz 2 → 1 |
