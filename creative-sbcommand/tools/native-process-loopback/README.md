# Windows process loopback helper

Standalone x64 .NET Framework console helper. It activates the Windows process-loopback virtual device for a selected PID and includes its child process tree. It does not capture the default output device or a microphone. Requires Windows build 20348 or later and .NET Framework 4.x.

## Build

Run `build.cmd`. It invokes `%WINDIR%\Microsoft.NET\Framework64\v4.0.30319\csc.exe` and produces `process-loopback.exe` beside the source; no SDK, NuGet package, driver, or administrator access is required.

## Protocol

* `--list`: stdout is a UTF-8 JSON array of objects with `pid`, `name`, and `has_window`. It lists inspectable processes, including background/windowless candidates, and excludes the listing helper itself. It is not restricted to Bilibili or any hard-coded application name. The browser prioritizes windowed candidates and supports name/PID search. It never reads window titles or command lines. Processes that disappear or cannot be inspected are skipped.
* `--probe`: stdout is a UTF-8 JSON object, for example `{"build":26300,"process_loopback_supported":true}`. Support is inferred from the OS build; this is not an activation test.
* `--pid N`: captures the selected process tree. stdout contains only interleaved stereo IEEE float32 little-endian PCM, with no header or log messages. stderr contains UTF-8 JSON lines. Once started, the helper emits `{"event":"ready","sample_rate":48000,"channels":2,"sample_format":"f32le","pid":N}`. A failure emits `{"event":"error","message":"...","hresult":"0x........"}` and exits nonzero. Data discontinuities are reported as `{"event":"discontinuity"}` on stderr. stderr and stdout must be drained concurrently.
* Send a line `stop` on stdin, close stdin, or let the target process exit to stop. The helper exits without terminating the target. A bounded output queue allows stopping even if the stdout reader stops consuming. Pending audio may be discarded on shutdown. If stdout is closed, the helper exits cleanly.

The helper requests 48 kHz float stereo with WASAPI automatic conversion. It currently has no 16-bit or 44.1 kHz fallback; an unsupported format returns the actual initialization HRESULT instead of reporting a different format. Capture is event-driven. Silent WASAPI buffers are emitted as zero samples, and no synthetic silence is generated when WASAPI supplies no packet. Consequently there is no independent wall-clock silence generator. A blocked stdout reader applies backpressure and can cause a capture discontinuity.

Activation is limited to 10 seconds. The process handle is opened for synchronization only. On exit, the audio client is stopped, COM objects and OS handles are released, and the helper does not alter the target process. Activation uses an MTA and an agile completion handler. The implementation follows Microsoft's [ApplicationLoopback sample](https://github.com/microsoft/Windows-classic-samples/tree/main/Samples/ApplicationLoopback) and [ActivateAudioInterfaceAsync documentation](https://learn.microsoft.com/en-us/windows/win32/api/mmdeviceapi/nf-mmdeviceapi-activateaudiointerfaceasync).

## Local verification

On Windows 11 x64 build 26300, `build.cmd`, `--probe`, and `--list` succeeded. A newly created silent PowerShell test process was used as the target: activation returned `ready` at 48000 Hz, stereo, f32le, and 775,680 captured bytes were all zero. Sending `stop`, closing stdin, and allowing a separate silent target to exit each led to helper exit code 0. `stop` and stdin closure also succeeded while stdout was deliberately left unread. No user application, microphone, or real Bilibili audio was captured, and no sound was played. Bilibili playback and listening quality remain untested.
