# 通用客户端 / 网页声音作为 EQ 输入

日期：2026-11-12（UTC+8）。本轮已实现并通过原生回环、流式传输、AudioWorklet及真实浏览器验证。并非只支持B站；按用户选择的进程PID工作。

## 客户端使用

1. 启动需要处理的客户端并播放声音。
2. 双击项目根目录 `start-eq-lab.cmd`，打开本地音频EQ实验室。如果旧版本服务还在运行，重新运行启动器会识别后端版本并打开新服务地址。
3. 点击“刷新客户端进程”，按程序名或PID搜索，选择目标并点“连接应用”。含窗口与后台候选；只读名称、PID、窗口存在标记，不读取窗口标题和命令行。
4. 等待“进程树采集已就绪”。这时默认不监听；点“开始监听”才输出，默认选择B EQ。
5. 选择EQ预设、扬声器/耳机预设模式与实际输出设备，可切A原音/B EQ对比。
6. “停止捕获”只停止本轮辅助采集进程，不终止目标客户端。换源或离开网页也会清理。

Windows原生模块支持进程及其子进程树，不使用麦克风，也不拿整张声卡混音冒充单应用。要求Windows build20348或更新，本机build26300已验证。当前原生模块请求48 kHz、双声道float32；初始化不支持时返回实际HRESULT，不伪造格式。受系统访问限制或未返回音频的程序不能保证有声音，页面会显示错误、等待、静音或实际信号状态。

## 原声与EQ输出的区别

捕获是复制原软件输出，不会自动接管/静音原软件。只听EQ时，可在Windows应用音量/输出设置中把原客户端送到不直出的虚拟通道（例如已安装的VoiceMeeter），再让本EQ页面输出到耳机。不要简单把原软件音量调0来消除叠音，因为也可能使捕获静音。未改变任何系统默认设备、应用音量或VoiceMeeter设置。

若选择浏览器根进程，可能同时包含本EQ页并造成回路。网页视频优先用标签页共享，或用另一浏览器承载EQ页面。

## 网页/标签页使用

点击“共享网页/系统音频”，在浏览器选择所需标签页（如B站）并勾选共享音频。仅共享窗口可能没有音频；返回没有audio轨道时，会停止本次全部轨道并报错。

只把audio送EQ，不绘制或上传画面。video轨道保留用于维持浏览器共享会话，停止时一起释放。原声抑制通过audio约束请求，但是否成功只按浏览器getSettings回报；未知或未启用会明确提示可能叠音。整屏系统音频不是单应用隔离。

## 本地端点与流格式

- `GET /api/session`：会话token及原生能力；token仅运行时在本地内存使用。
- `GET /api/processes`：`available / processes[{pid,name,has_window}] / reason`。
- `GET /api/process-audio?pid=N&token=...`：WebSocket音频流，校验随机token、当前localhost Origin与Host，升级前拒绝非法访问。
- text：helper的ready/error/ended诊断；ready声明真实sample_rate、channels=2、sample_format=f32le。
- binary：交错双声道little-endian float32。保证完整8字节帧，元数据ready前不发送PCM。
- close/stop：关闭辅助helper、管道和浏览器工作节点，不关闭目标程序；没有音频包时继续等待，不使用5秒无包超时误断暂停的软件。

服务只监听127.0.0.1，不接受音频上传；日志对会话token脱敏。用户本地文件仍在浏览器解码。

## 缓冲与验证

AudioWorklet采用约30 ms预缓冲，上限0.5秒及256块，欠载补零且不推进空队列游标，超限丢旧；统计按约100 ms节流。采样率不同时采用线性插值，属于试听实现，不等于专业抗混叠重采样器。

结果见：

- `audio/browser-validation.json`：真实Chrome回归、原生静音目标进程到网页链路、合成共享流、无音频/取消/晚到结果清理；未采集用户已有应用或真实屏幕。
- `audio/process-validation.json`：真实原生API、PCM吞吐、完整帧、token/Origin/Host拒绝、碎片化ping、停止不杀目标；另用无音频模拟helper检查等待与关闭。
- `audio/worklet-validation.json`：欠载恢复、有限缓冲、非有限输入防护、三种采样率转换、停止工作节点。

复现脚本位于tools目录：`verify-eq-audio-browser.py`、`verify-process-capture.py`、`verify-process-worklet.cjs`。原生源码及build.cmd位于tools/native-process-loopback，运行依赖process-loopback.exe已编译保留。真实各客户端的声音、权限与听感仍需用户选择目标后确认；不能由静音自测推断所有软件都逐一验收。
