# 离线参数契约证据索引

主报告：../parameter-ranges.md，机器可读：../parameter-ranges.json。

- sources.md：来源路径、大小、SHA256、程序集/工具版本。
- ranges-ui-constants.tsv：Devices中的56个范围构造点；实际制表符分隔，行号对应反编译C#。
- il-offsets.txt：关键IL指令、方法与offset。
- snippets/：定点片段；call-chains.md：调用链摘要。
- ../call-chain-crosscheck/：主对话独立提取的完整类与可复现命令。
- ../factory-data-audit.md/json：预设双模式与混响记录类型的独立核验。

## 复现范围常量

运行项目内scripts/reproduce-contracts.py，使用现有ILSpy工具对本机文件静态反编译，在项目内临时目录处理后自动删除中间树，只保留范围文本和来源清单。不会加载或执行目标DLL。

## 预设复核脚本

出厂数据已由主对话逐文件对照，原始预设仍位于sources.md所列ProgramData目录。对照口径为每种输出模式的PreAmp、全部十段Frequency/Value和SBX Settings；混响按FeatureId/ParamId/Type/Value逐条对照。factory-data-audit.json包含结果与输入哈希。

## 边界

范围配置和Granularity是框架声明，不代替硬件GetParamInfo实测。禁止把预设极值当允许范围，也不能以关键词未命中证明算法不存在。

复现结果：Devices 56处构造位置与TSV的文件/行号集合完全相同；另在UI Framework发现1处动态范围构造，因此跨程序集文本搜索共57行，不能把它们都算作静态常量。
