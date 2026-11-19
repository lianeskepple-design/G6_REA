# 静态分析来源与工具

全部目标以文件形式读取，未加载厂商代码。

| 文件 | 大小 | 版本 | SHA256 |
|---|---:|---|---|
| C:\Program Files (x86)\Creative\Sound Blaster Command\Platform\Creative.Platform.Devices.dll | 647680 | 2.4.67.0 | 9337d9cabc9c5de4a5026833adf308115885b1b9d884928fae24edbfaf736e9f |
| C:\Program Files (x86)\Creative\Sound Blaster Command\Platform\Creative.Platform.Mixer.dll | 222720 | 2.4.67.0 | e1f1cdcd2d79e8ba006912f76c263b2dad0c04407875d3cc58e7c324d63eb88e |
| C:\Program Files (x86)\Creative\Sound Blaster Command\Platform\Interop.CtSndCr.dll | 34816 | 1.0.0.72 | 7956f40c9b67957ab9d7a924099217ce2d009a703bd8e8c211d7fef816154dbd |
| C:\Program Files (x86)\Creative\Sound Blaster Command\Platform\Creative.SBConnect.UI.Framework.dll | 993792 | — | c6359888f96fde91aa48073e652bfbf462a47e3d8de88b7d7b60f21c1fb2f4b3 |
| C:\Program Files (x86)\Creative\Sound Blaster Command\Platform\SndCrUSB.DLL | 3988480 | — | 564069833549063cda80f14f27d2e5e3f33edbabdfaf5e31b01eb551c77959fe |
| C:\ProgramData\Creative\SBCommand\Product\AE9\Presets\EQ\*.json | 35 | — | 目录中的逐文件哈希见factory-data-audit.json |
| C:\ProgramData\Creative\SBCommand\Product\G6\Presets\EQ\*.json | 34 | — | 目录中的逐文件哈希见factory-data-audit.json |
| C:\ProgramData\Creative\SBCommand\Product\AE9\Profiles\*.json | 25 | — | 目录中的逐文件哈希见factory-data-audit.json |
| C:\ProgramData\Creative\SBCommand\Product\G6\Profiles\*.json | 23 | — | 目录中的逐文件哈希见factory-data-audit.json |
| C:\ProgramData\Creative\SBCommand\Product\G6\Creative.SBConnect2.G6.dll | 139760 | — | f24f56bf3747ad377b3d28c8e53d8ea6447c817c35605330d4bff9fbd1f78590 |

工具：ilspycmd 9.1.0.7988、现有.NET9 Major roll-forward、Python3.12.10（dnfile/pefile）。
产品预设的逐文件数据核验与来源哈希另见 ../factory-data-audit.json。