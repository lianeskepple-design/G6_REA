# 原厂数据离线复核

日期：2026-11-12（UTC+8）。仅本机静态文件分析，未运行厂商代码或操作音频设备。

- G6：23组SBX profile、34组EQ；导出设置与原厂逐项一致。
- AE-9：25组profile设置导出与原厂一致，安装目录有35组EQ。
- G6 EQ有18组Speaker/Headphone不同。29组跨设备同名EQ，Speaker有6组、Headphone有7组不同。
- FPS/CallofDutySeries/DrivingSimulation仅Speaker全零，耳机均有真实增益。
- 两个G6 JSON的设置和order一致；合并版省略type/speaker_method元字段，属于结构差异。
- 混响XML与安装目录SndCrUSB.DLL在0x9738的字节完全一致；XML与JSON逐项一致。
- 19组混响每组32条，共608条：437 Float、152 Bool、19 Integer。
- 每组为参数1至31加258（ActivePreset），不包含Enable=0。并非608个浮点值或32个独立混响DSP旋钮。

源文件SHA256、比较口径与结果在 factory-data-audit.json。比较覆盖每个输出模式的PreAmp、十段频点和增益。

原厂频点和增益不能单独决定滤波器Q、阶数或实现结构。关键词未命中不能证明算法不存在；G6无混响、DSP全在硬件、Scout节点严格串联等旧推断均不能作为已证实结论。
