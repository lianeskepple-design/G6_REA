# 通用客户端进程／网页输出作为 EQ 输入

日期：2026-11-12（UTC+8）。已实现并通过原生采集、传输、缓冲与浏览器回归验证。

## 已交付

- 通用客户端进程选择与名称/PID搜索，不限定B站；后台进程也可列出。
- Windows WASAPI进程回环，包含目标子进程树；不使用麦克风，不用整卡混音冒充单应用。
- `tools/native-process-loopback/process-loopback.exe`已编译，保留Program.cs、build.cmd、README。
- localhost服务经token/Origin/Host校验传送PCM，AudioWorklet有界缓冲、欠载恢复与线性采样率转换。
- 用户点击才连接；默认不监听，点击开始监听后进入原音A/EQ B链。停止只关采集辅助模块，不关闭目标客户端。
- 网页标签页/系统声音共享为另一入口；无音频、取消、晚到结果和停止共享会释放全部轨道。共享画面不绘制、不上传。
- 页面显示等待、静音/极低电平、实际信号以及缓冲/欠载状态。

## 使用

双击项目根目录start-eq-lab.cmd → 外部音频捕获 → 刷新客户端进程 → 搜索应用名/PID → 连接应用 → 开始监听。

详细说明与端点：[eq-curves/audio-capture-contract.md](eq-curves/audio-capture-contract.md)。

采集是复制原声，不自动静音或接管原软件。只听EQ时应把原客户端路由到不直出的虚拟通道（可用已有VoiceMeeter），本页输出到耳机。未修改系统默认设备、原软件音量或VoiceMeeter配置。

## 验证与边界

本机Windows11 x64 build26300支持。原生真实自测只针对本轮新建的静音进程，验证实际48 kHz双声道f32le、PCM帧/吞吐、停止辅助模块且目标仍存活；另用模拟无音频helper验证等待与关闭。浏览器验证覆盖原生PCM进网页、34个预设的双模式EQ、模拟共享流和旧功能回归。

证据：eq-curves/audio/browser-validation.json、process-validation.json、worklet-validation.json。真实各客户端发声、权限与听感仍需用户选择后试听，不能声称所有应用都逐一实际验过。受系统权限或保护限制的目标可能无法返回音频；当前原生模块只请求48 kHz float，不支持时报告真实HRESULT。
