# G6 离线 EQ 模式与频响图册

日期：2026-11-12（UTC+8）。原始参考文件与产品程序未修改。

## 文件入口

- [本地音频与输入输出设备版本](LOCAL-AUDIO.md)：先双击项目根目录的[start-eq-lab.cmd](../../start-eq-lab.cmd)，在localhost页面选择自己的音频或麦克风，实时A/B。
- [预制音频与四组 A→B 示例](audio/README.md)；[网页验收结果](audio/browser-validation.json)。
- [交互图册](index.html)：搜索预设、扬声器/耳机叠加、切换44.1/48/96k、附加增益、悬停读数、当前曲线CSV。
- [完整PDF](all-modes-catalog.pdf)：34页，每页一个预设，双模式频响与十段原厂值。
- [34模式总览PNG](all-modes-overview.png)；[脚步增强示例PNG](footstep-enhancer.png)。
- [68行原厂增益CSV](eq-modes-and-gains.csv)：34预设×2模式，UTF-8 BOM，便于Excel导入。
- [48k连续频响CSV](eq-frequency-response-48k.csv)：68曲线×1024频点，共69632行数据。
- [结构化数据](eq-catalog-data.json)；[来源哈希与曲线峰值摘要](manifest.json)。

## 读图口径

原厂十段频点：31 / 62 / 125 / 250 / 500 / 1000 / 2000 / 4000 / 8000 / 16000 Hz。离散点为JSON原厂增益；平滑线为当前sbc-eq的软件理论频响，默认48 kHz、Q=1、附加增益0 dB并计入预设PreAmp。相邻频段叠加，不能把滑块值连线当成实际频响。

34组预设共68条输出模式记录，其中18组双模式不同、16组相同。中文名与分类为整理译名，英文ID才是CLI参数。这里是EQ预设：相同名字的SBX profile是另一套效果参数；FootstepEnhancer不等于Scout。

## 模式索引

| 中文整理名 | 原厂ID / CLI参数 | 分类 | 扬声器/耳机 |
|---|---|---|---|
| 原声 | Acoustic | 音乐风格 | 相同 |
| 音乐 | Music | 通用与增强 | 相同 |
| 古典 | Classical | 音乐风格 | 相同 |
| 电影 | Movie | 通用与增强 | 相同 |
| 乡村 | Country | 音乐风格 | 相同 |
| 脚步增强 | FootstepEnhancer | 通用与增强 | 相同 |
| 舞曲 | Dance | 音乐风格 | 相同 |
| 平直 | Flat | 通用与增强 | 相同 |
| 嘻哈 | HipHop | 音乐风格 | 相同 |
| 爵士 | Jazz | 音乐风格 | 相同 |
| 流行 | Pop | 音乐风格 | 相同 |
| R&B | RnB | 音乐风格 | 相同 |
| 摇滚 | Rock | 音乐风格 | 相同 |
| 人声 | Vocal | 音乐风格 | 相同 |
| 游戏通用 | Gaming | 游戏与场景 | 相同 |
| 动作与冒险 | AdventureAndAction | 游戏与场景 | 不同 |
| Apex 英雄 | ApexLegend | 游戏与场景 | 不同 |
| 传说对决 | ArenaOfValor | 游戏与场景 | 不同 |
| 使命召唤系列 | CallofDutySeries | 游戏与场景 | 不同 |
| CS:GO | CounterStrikeGlobalOffensive | 游戏与场景 | 不同 |
| Dota 2 | Dota2 | 游戏与场景 | 不同 |
| 驾驶模拟 | DrivingSimulation | 游戏与场景 | 不同 |
| 堡垒之夜 | Fortnite | 游戏与场景 | 不同 |
| 第一人称射击 | FPS | 游戏与场景 | 不同 |
| 英雄联盟 | LeagueOfLegends | 游戏与场景 | 不同 |
| 合金装备 V：幻痛 | MetalGearSolidVThePhantomPain | 游戏与场景 | 不同 |
| 守望先锋 | Overwatch | 游戏与场景 | 不同 |
| PUBG / 绝地求生 | PlayerUnknown | 游戏与场景 | 不同 |
| 赛车计划 | ProjectCars | 游戏与场景 | 不同 |
| 即时战略 | RealTimeStrategy | 游戏与场景 | 不同 |
| 火箭联盟 | RocketLeague | 游戏与场景 | 不同 |
| 角色扮演 | RPG | 游戏与场景 | 相同 |
| 体育场 | Stadium | 游戏与场景 | 不同 |
| 巫师 3：狂猎 | TheWitcher3WildHunt | 游戏与场景 | 不同 |

## 特别容易混淆的三组

FPS、CallofDutySeries、DrivingSimulation只有Speaker曲线全零，Headphone均非零。完整原厂增益已分别导出，不能按名字认定为无效果。

## 复现与验证

在creative-sbcommand目录生成：`./tools/audio-env/Scripts/python.exe -X utf8 ./tools/build-eq-catalog.py`；验证：`./tools/audio-env/Scripts/python.exe -X utf8 ./tools/validate-eq-catalog.py`。

全部68条曲线已与Rust处理器实际导出的131072帧脉冲FFT逐一对照，最大绝对响应差为`6.405480164062283e-7 dB`。HTML计算在44.1/48/96 kHz下共2448个检查点与SciPy对照，最大差`3.0820679342014046e-10 dB`。原厂CSV的68行、连续频响CSV的69632行及PDF的34页均已检查。完整结果见[validation.json](validation.json)。

原厂Q、系数拓扑、硬件执行和频响未证实。本图册是当前软件EQ模型的理论频响，不是声卡实测，也不是完整SBX效果。
