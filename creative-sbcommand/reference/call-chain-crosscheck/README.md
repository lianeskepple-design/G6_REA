# 参数调用链独立交叉检查

日期：2026-11-12（UTC+8）。静态读取 Creative.Platform.Devices.dll，未加载/执行其代码，未访问音频设备。

来源与 SHA256、工具版本、完整可复现命令在 sources.json。证据为同目录定点反编译片段；CLI已加 --disable-updatecheck。

## 已确认的封装范围

`EffectParameterRangeConfig<T>` 构造器顺序为 `(min, max, granularity, defaultVal)`；不是从参数值顺序猜测。

| 参数 | 最小 | 最大 | Granularity | 默认 | 具体证据 |
|---|---:|---:|---:|---:|---|
| 图形EQ单段增益 | -24 | 24 | 1 | 0 | GraphicsEqFeatureEnricher.EnrichUndetermined |
| 图形EQ前级 | -12 | 12 | 1 | 0 | 同上 |
| XBass分频点，Hz | 10 | 1000 | 1 | 80 | XBassXFi23FeatureEnricher.EnrichUndetermined |
| BassManagement分频点，Hz | 10 | 1000 | 1 | 80 | 同上，仅HasBassManagement为true时添加 |
| XBass强度（归一化分支） | 0 | 1 | 0.01 | 0.5 | SoundCoreRepository且ProductCodeName不为Kratos2 |
| XBass强度（百分数分支） | 0 | 100 | 1 | 50 | 其余分支，以及CDC+PropertyStore路径 |

这些是平台封装的配置范围，并非读取实体G6后返回的实际能力。BasicEffectParameter的边界检查只比较Min/Max，没有在所读方法中强制Granularity量化；不能把Granularity=1解释成所有API写入都必须为整数。UI显示范围和步进还需另外追踪。

`GraphicsEqFeature.GetEqBandFrequencies()` 返回固定十段：31、62、125、250、500、1000、2000、4000、8000、16000 Hz。原厂预设与这份代码一致。

## 对G6的明确分支

`XBassXFi23FeatureEnricher.HasBassManagement` 直接返回 `target.DeviceEndpoint.ProductCodeName != "G6"`。因此此封装中G6不会走添加BassManagement分频和SubwooferGain参数的分支。XBass分频参数依然有定义，不能把XBass和扬声器低音管理混为一谈，也不能据此判断G6硬件绝无相关内部处理。

## Scout与混响

`SoundCoreScoutFeature.Enable()` 保存Surround、Crystalizer、XBass、SVM、DialogPlus、GraphicsEQ当前开关，再写Scout总开关；重载版本先写这六项状态再开启Scout。这里没有脚步识别算法、十段EQ曲线或节点顺序，旧“Scout=固定串联链”的说法证据不足。

`ImmersionReverbAlgo` 确实包含参数映射逻辑：ShouldSetReverb比较surroundLevel与thresholdPct*(Max-Min)；GetReverbValue把超过阈值的surroundLevel线性映射到EAX混响范围的一部分。这是上层控制值映射，不能当作混响波形生成算法。实际调用分支、阈值常量以及设备支持状况仍须沿调用方核实。

## 仍未知

实体设备运行时能力、原厂EQ滤波Q和拓扑、Scout实际频响、混响网络、APO与固件的实际分工尚未验证。这些结论不由字符串命中或参数枚举直接决定。
