using System;
using System.Collections.Generic;
using System.Collections.Concurrent;
using System.Diagnostics;
using System.Globalization;
using System.IO;
using System.Runtime.InteropServices;
using System.Text;
using System.Threading;

[StructLayout(LayoutKind.Sequential, Pack=2)]
struct WaveFormat {
    public ushort tag, channels;
    public uint rate, bytesPerSecond;
    public ushort blockAlign, bits, extra;
}
[StructLayout(LayoutKind.Sequential)]
struct ActivationParams { public int type; public uint pid; public int mode; }
[StructLayout(LayoutKind.Explicit, Size=24)]
struct PropVariant { [FieldOffset(0)] public ushort vt; [FieldOffset(8)] public int size; [FieldOffset(16)] public IntPtr data; }
[StructLayout(LayoutKind.Sequential, CharSet=CharSet.Unicode)]
struct OsVersion { public int size, major, minor, build, platform; [MarshalAs(UnmanagedType.ByValTStr, SizeConst=128)] public string servicePack; }

[ComImport, Guid("1CB9AD4C-DBFA-4C32-B178-C2F568A703B2"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IAudioClient {
    [PreserveSig] int Initialize(int share, uint flags, long duration, long periodicity, ref WaveFormat format, IntPtr session);
    [PreserveSig] int GetBufferSize(out uint frames);
    [PreserveSig] int GetStreamLatency(out long latency);
    [PreserveSig] int GetCurrentPadding(out uint padding);
    [PreserveSig] int IsFormatSupported(int share, ref WaveFormat format, out IntPtr closest);
    [PreserveSig] int GetMixFormat(out IntPtr format);
    [PreserveSig] int GetDevicePeriod(out long defaultPeriod, out long minimumPeriod);
    [PreserveSig] int Start();
    [PreserveSig] int Stop();
    [PreserveSig] int Reset();
    [PreserveSig] int SetEventHandle(IntPtr handle);
    [PreserveSig] int GetService(ref Guid iid, [MarshalAs(UnmanagedType.IUnknown)] out object service);
}
[ComImport, Guid("C8ADBD64-E71E-48A0-A4DE-185C395CD317"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface IAudioCaptureClient {
    [PreserveSig] int GetBuffer(out IntPtr data, out uint frames, out uint flags, out ulong devicePosition, out ulong qpcPosition);
    [PreserveSig] int ReleaseBuffer(uint frames);
    [PreserveSig] int GetNextPacketSize(out uint frames);
}
[ComImport, Guid("72A22D78-CDE4-431D-B8CC-843A71199B6D"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
public interface IActivateAudioInterfaceAsyncOperation {
    [PreserveSig] int GetActivateResult(out int result, [MarshalAs(UnmanagedType.IUnknown)] out object activated);
}
[ComVisible(true), Guid("41D949AB-9862-444A-80F6-C261334DA5EB"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
public interface IActivateAudioInterfaceCompletionHandler {
    [PreserveSig] int ActivateCompleted(IActivateAudioInterfaceAsyncOperation operation);
}
[ComVisible(true), Guid("94EA2B94-E9CC-49E0-C0FF-EE64CA8F5B90"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
public interface IAgileObject { }
[ComVisible(true), ClassInterface(ClassInterfaceType.None)]
public sealed class Completion : IActivateAudioInterfaceCompletionHandler, IAgileObject {
    public readonly ManualResetEvent Done = new ManualResetEvent(false);
    public int Result = unchecked((int)0x80004005);
    public object Client;
    public int ActivateCompleted(IActivateAudioInterfaceAsyncOperation operation) {
        try { int hr = operation.GetActivateResult(out Result, out Client); if (hr < 0) Result = hr; }
        catch (Exception ex) { Result = Marshal.GetHRForException(ex); }
        finally { Done.Set(); }
        return 0;
    }
}
class Program {
    [DllImport("Mmdevapi.dll", CharSet=CharSet.Unicode, ExactSpelling=true)]
    static extern int ActivateAudioInterfaceAsync(string device, ref Guid iid, ref PropVariant parameters, IActivateAudioInterfaceCompletionHandler handler, out IActivateAudioInterfaceAsyncOperation operation);
    [DllImport("ntdll.dll")] static extern int RtlGetVersion(ref OsVersion version);
    [DllImport("combase.dll")] static extern int RoInitialize(uint type);
    [DllImport("combase.dll")] static extern void RoUninitialize();
    [DllImport("kernel32.dll", SetLastError=true)] static extern IntPtr OpenProcess(uint access, bool inherit, int pid);
    [DllImport("kernel32.dll", SetLastError=true)] static extern uint WaitForSingleObject(IntPtr handle, uint milliseconds);
    [DllImport("kernel32.dll")] static extern bool CloseHandle(IntPtr handle);
    static readonly Guid AudioClientId = new Guid("1CB9AD4C-DBFA-4c32-B178-C2F568A703B2");
    static readonly Guid CaptureId = new Guid("C8ADBD64-E71E-48A0-A4DE-185C395CD317");
    static volatile bool stopping;
    static string Quote(string s) {
        StringBuilder b = new StringBuilder("\"");
        foreach (char c in s ?? "") { if (c == '\\' || c == '"') b.Append('\\').Append(c); else if (c < 32) b.Append("\\u").Append(((int)c).ToString("x4")); else b.Append(c); }
        return b.Append('"').ToString();
    }
    static void Check(int hr, string context) { if (hr < 0) throw new AudioException(context, hr); }
    sealed class AudioException : Exception { public AudioException(string message, int hr) : base(message) { HResult = hr; } }
    static void Error(Exception ex) {
        int hr = Marshal.GetHRForException(ex);
        Console.Error.WriteLine("{\"event\":\"error\",\"message\":" + Quote(ex.Message) + ",\"hresult\":\"0x" + ((uint)hr).ToString("X8") + "\"}");
    }
    static int Probe() {
        OsVersion v = new OsVersion(); v.size = Marshal.SizeOf(typeof(OsVersion));
        Check(RtlGetVersion(ref v), "RtlGetVersion failed");
        Console.WriteLine("{\"build\":" + v.build + ",\"process_loopback_supported\":" + (v.build >= 20348 ? "true" : "false") + "}");
        return 0;
    }
    static int List() {
        List<string> items = new List<string>();
        int self; using (Process current = Process.GetCurrentProcess()) self = current.Id;
        foreach (Process p in Process.GetProcesses()) {
            try {
                string name = p.ProcessName;
                bool window = p.MainWindowHandle != IntPtr.Zero;
                if (p.Id > 0 && p.Id != self)
                    items.Add("{\"pid\":" + p.Id + ",\"name\":" + Quote(name) + ",\"has_window\":" + (window ? "true" : "false") + "}");
            } catch (Exception) { } finally { p.Dispose(); }
        }
        Console.WriteLine("[" + String.Join(",", items.ToArray()) + "]"); return 0;
    }
    static WaveFormat Format(int rate, bool floating) {
        WaveFormat w = new WaveFormat(); w.tag = (ushort)(floating ? 3 : 1); w.channels = 2; w.rate = (uint)rate;
        w.bits = (ushort)(floating ? 32 : 16); w.blockAlign = (ushort)(w.channels * w.bits / 8); w.bytesPerSecond = w.rate * w.blockAlign; return w;
    }
    static int Capture(int pid) {
        if (pid <= 0) throw new ArgumentException("PID must be positive");
        IntPtr target = OpenProcess(0x00100000, false, pid);
        if (target == IntPtr.Zero) throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error(), "Cannot open target process for synchronization");
        Completion completion = new Completion();
        IActivateAudioInterfaceAsyncOperation operation = null;
        IAudioClient client = null; IAudioCaptureClient capture = null;
        AutoResetEvent sampleEvent = null; bool started = false;
        IntPtr blob = IntPtr.Zero;
        try {
            ActivationParams ap = new ActivationParams(); ap.type = 1; ap.pid = (uint)pid; ap.mode = 0;
            blob = Marshal.AllocCoTaskMem(Marshal.SizeOf(typeof(ActivationParams))); Marshal.StructureToPtr(ap, blob, false);
            PropVariant pv = new PropVariant(); pv.vt = 65; pv.size = Marshal.SizeOf(typeof(ActivationParams)); pv.data = blob;
            Guid iid = AudioClientId;
            Check(ActivateAudioInterfaceAsync("VAD\\Process_Loopback", ref iid, ref pv, completion, out operation), "ActivateAudioInterfaceAsync failed");
            if (!completion.Done.WaitOne(10000)) throw new AudioException("Audio activation timed out", unchecked((int)0x800705B4));
            Check(completion.Result, "Process loopback activation failed");
            client = (IAudioClient)completion.Client;
            WaveFormat format = Format(48000, true);
            const uint flags = 0x00020000 | 0x00040000 | 0x80000000;
            int hr = client.Initialize(0, flags, 0, 0, ref format, IntPtr.Zero);
            Check(hr, "IAudioClient.Initialize (48000 Hz float stereo) failed");
            object service; iid = CaptureId; Check(client.GetService(ref iid, out service), "GetService(IAudioCaptureClient) failed");
            capture = (IAudioCaptureClient)service;
            sampleEvent = new AutoResetEvent(false);
            Check(client.SetEventHandle(sampleEvent.SafeWaitHandle.DangerousGetHandle()), "SetEventHandle failed");
            Check(client.Start(), "IAudioClient.Start failed"); started = true;
            Thread input = new Thread(delegate() { try { string line; while ((line = Console.ReadLine()) != null) { if (line.Trim().Equals("stop", StringComparison.OrdinalIgnoreCase)) break; } } catch (IOException) { } finally { stopping = true; } });
            input.IsBackground = true; input.Start();
            Console.Error.WriteLine("{\"event\":\"ready\",\"sample_rate\":" + format.rate + ",\"channels\":2,\"sample_format\":\"f32le\",\"pid\":" + pid + "}");
            Stream output = Console.OpenStandardOutput();
            BlockingCollection<byte[]> packets = new BlockingCollection<byte[]>(64);
            Thread writer = new Thread(delegate() {
                try { foreach (byte[] packet in packets.GetConsumingEnumerable()) output.Write(packet, 0, packet.Length); }
                catch (IOException) { stopping = true; }
                catch (ObjectDisposedException) { stopping = true; }
            });
            writer.IsBackground = true; writer.Start();
            while (!stopping) {
                uint wait = WaitForSingleObject(target, 0);
                if (wait == 0) break;
                if (wait == 0xFFFFFFFF) throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error(), "Target process wait failed");
                sampleEvent.WaitOne(100);
                if (stopping) break;
                uint frames;
                Check(capture.GetNextPacketSize(out frames), "GetNextPacketSize failed");
                while (frames != 0 && !stopping) {
                    IntPtr data; uint flagsOut; ulong pos, qpc; uint acquired;
                    Check(capture.GetBuffer(out data, out acquired, out flagsOut, out pos, out qpc), "GetBuffer failed");
                    byte[] bytes;
                    try {
                        bytes = new byte[checked((int)(acquired * 8))];
                        if ((flagsOut & 2) == 0 && bytes.Length != 0) Marshal.Copy(data, bytes, 0, bytes.Length);
                    } finally { Check(capture.ReleaseBuffer(acquired), "ReleaseBuffer failed"); }
                    if ((flagsOut & 1) != 0) Console.Error.WriteLine("{\"event\":\"discontinuity\"}");
                    while (!stopping && !packets.TryAdd(bytes, 100)) {
                        uint state = WaitForSingleObject(target, 0);
                        if (state == 0) stopping = true;
                        else if (state == 0xFFFFFFFF) throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error(), "Target process wait failed");
                    }
                    Check(capture.GetNextPacketSize(out frames), "GetNextPacketSize failed");
                }
            }
            return 0;
        } finally {
            if (started && client != null) client.Stop();
            if (capture != null && Marshal.IsComObject(capture)) Marshal.ReleaseComObject(capture);
            if (client != null && Marshal.IsComObject(client)) Marshal.ReleaseComObject(client);
            if (operation != null && Marshal.IsComObject(operation)) Marshal.ReleaseComObject(operation);
            if (sampleEvent != null) sampleEvent.Dispose();
            if (blob != IntPtr.Zero) Marshal.FreeCoTaskMem(blob);
            CloseHandle(target);
            GC.KeepAlive(completion);
        }
    }
    [MTAThread] static int Main(string[] args) {
        Console.OutputEncoding = new UTF8Encoding(false);
        try {
            if (args.Length == 1 && args[0] == "--probe") return Probe();
            if (args.Length == 1 && args[0] == "--list") return List();
            int pid;
            if (args.Length == 2 && args[0] == "--pid" && Int32.TryParse(args[1], NumberStyles.None, CultureInfo.InvariantCulture, out pid)) {
                Check(RoInitialize(1), "RoInitialize failed");
                try { return Capture(pid); } finally { RoUninitialize(); }
            }
            throw new ArgumentException("Usage: process-loopback.exe --list | --probe | --pid N");
        } catch (Exception ex) { Error(ex); return 1; }
    }
}
