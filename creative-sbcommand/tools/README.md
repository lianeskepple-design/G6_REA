# Creative G6 分析工具

安装与验证日期：2026-11-12（UTC+8）。仅安装工具与检查环境，未抓包、未录音、未播放测试信号，未修改声卡设置。

| 工具 | 版本 | 路径 |
|---|---|---|
| Wireshark / tshark | 4.6.9 | C:/Program Files/Wireshark/ |
| USBPcap / USBPcapCMD | 1.5.4.0 | C:/Program Files/USBPcap/ |
| ILSpy GUI（独立运行版） | 11.2.0.10050 | ILSpy/ILSpy.exe |
| ILSpy CLI | 9.1.0.7988 | ilspycmd.cmd |
| 音频测量 Python | 3.12.10 | audio-env/Scripts/python.exe |

Python 包括 numpy、scipy、sounddevice、soundfile、matplotlib、dnfile；依赖版本见 requirements-audio.lock.txt，验证结果见 verification.json。

## 验证结果

- 系统安装包签名有效，两个安装器均返回 0。
- tshark 输出版本成功；USBPcapCMD 输出帮助成功。
- ILSpy GUI 签名有效，启动并响应；CLI 输出版本成功。
- WAV 内存读写、SciPy 滤波、Matplotlib 内存绘图通过。
- PortAudio 枚举到 WASAPI、MME、DirectSound、WDM-KS。

## USB 接口待验证

安装后 USBPcap 驱动状态为 Stopped，dumpcap 暂未发现可捕获接口。方便时重启 Windows，再连接 G6 检查 USBPcap 接口。当前不能标记为 USB 实际抓包已通过。未安装网络流量捕获用 Npcap，G6 USB 分析使用 USBPcap。本轮未重启电脑。

## PowerShell 命令（从 creative-sbcommand 根目录执行）

```powershell
& './tools/ILSpy/ILSpy.exe'
& './tools/ilspycmd.cmd' --version
& './tools/audio-env/Scripts/python.exe' -m pip check
& 'C:/Program Files/Wireshark/tshark.exe' --version
& 'C:/Program Files/USBPcap/USBPcapCMD.exe' --help
```

CLI 使用现有 .NET 9 的 Major roll-forward，无需完整 SDK；GUI 自带运行时。未修改系统级环境变量。

如果 pip 卡在缓存临时文件创建，使用 --no-cache-dir，并把当前安装进程的 TEMP/TMP 指向项目内可写目录。

来源：Wireshark 官方下载、desowin/usbpcap 官方 GitHub Release、icsharpcode/ILSpy 官方 GitHub Release、NuGet 官方 ilspycmd 包、PyPI。
