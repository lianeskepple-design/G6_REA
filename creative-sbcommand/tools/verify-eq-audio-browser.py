"""Verify both EQ pages using Chrome CDP, muted output and virtual capture devices."""
from pathlib import Path
import importlib.util
import json
import shutil
import socket
import subprocess
import tempfile
import threading
import sys
import time
import urllib.parse
import urllib.request

ROOT=Path(__file__).resolve().parent.parent
CAT=ROOT/"tools/eq-curves"
BROWSER=Path("C:/Program Files/Google/Chrome/Application/chrome.exe")
NODE=Path("C:/Program Files/nodejs/node.exe")

spec=importlib.util.spec_from_file_location("eq_test_server",ROOT/"tools/serve-eq-lab.py")
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
module.Handler.log_message=lambda *args:None
original_popen=subprocess.Popen
capture_target=original_popen([getattr(sys,'_base_executable',sys.executable),'-c','import time; time.sleep(120)','eq-owned-silent-target'],creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
helpers=[]
def tracked_popen(*args,**kwargs):
    child=original_popen(*args,**kwargs)
    command=args[0] if args else kwargs.get('args',[])
    if isinstance(command,(list,tuple)) and len(command)>2 and str(command[0])==str(module._process_audio.HELPER) and command[1]=='--pid':helpers.append(child)
    return child
subprocess.Popen=tracked_popen
server=module.create_server(0)
thread=threading.Thread(target=server.serve_forever,daemon=True);thread.start()
base=f"http://127.0.0.1:{server.server_port}"
temporary=Path(tempfile.mkdtemp(prefix=".eq-browser-cdp-",dir=ROOT/"tools"))
profile=temporary/"profile"
profile.mkdir()
with socket.socket() as reserve:
    reserve.bind(('127.0.0.1',0)); debug_port=reserve.getsockname()[1]
command=[str(BROWSER),"--headless=new","--no-sandbox","--disable-gpu","--disable-software-rasterizer","--no-first-run","--no-default-browser-check","--mute-audio","--disable-audio-output","--allow-file-access-from-files","--disable-background-networking","--disable-component-update","--disable-sync","--use-fake-device-for-media-stream","--use-fake-ui-for-media-stream","--autoplay-policy=no-user-gesture-required","--remote-debugging-address=127.0.0.1",f"--remote-debugging-port={debug_port}","--enable-logging=stderr",f"--user-data-dir={profile}","about:blank"]
process=None
try:
    log=open(temporary/'browser-stderr.log','w',encoding='utf-8')
    process=subprocess.Popen(command,stdout=subprocess.DEVNULL,stderr=log)
    deadline=time.monotonic()+12
    port=debug_port
    while True:
        try:
            with urllib.request.urlopen(f'http://127.0.0.1:{port}/json/version',timeout=.4) as status:
                json.load(status)
            break
        except Exception:
            if time.monotonic()>deadline:
                log.close()
                details=(temporary/'browser-stderr.log').read_text(encoding='utf-8',errors='replace')[-2000:]
                raise RuntimeError(f'Chrome test endpoint unavailable (exit={process.poll()}): '+details)
            time.sleep(.1)
    request=urllib.request.Request(f"http://127.0.0.1:{port}/json/new?about:blank",method="PUT")
    with urllib.request.urlopen(request,timeout=5) as response:target=json.load(response)
    html=(CAT/"index.html").read_text(encoding="utf-8")
    function=html[html.index("function response("):html.index("function options(")]
    config={"capturePid":capture_target.pid,"websocket":target["webSocketDebuggerUrl"],"base":base,"fixture":str((CAT/"audio/wavs/original.wav").resolve()),"catalog":json.loads((CAT/"eq-catalog-data.json").read_text(encoding="utf-8")),"responseFunction":function,"output":str(CAT/"audio/browser-validation.json")}
    result=subprocess.run([str(NODE),str(ROOT/"tools/verify-eq-browser.cjs")],input=json.dumps(config,ensure_ascii=False),capture_output=True,text=True,encoding="utf-8",errors="replace",timeout=100)
    print(result.stdout,flush=True)
    if result.returncode:raise RuntimeError(result.stderr or "Browser assertions failed; see audio/browser-validation.json")
    for helper in helpers:helper.wait(timeout=3)
    record_path=CAT/'audio/browser-validation.json'
    record=json.loads(record_path.read_text(encoding='utf-8'))
    record['native_target_alive_after_stop']=capture_target.poll() is None
    record['native_capture_helpers_started']=len(helpers)
    record['native_capture_helpers_all_exited']=all(helper.poll() is not None for helper in helpers)
    if not (record['native_target_alive_after_stop'] and len(helpers)>=1 and record['native_capture_helpers_all_exited']):raise RuntimeError('Native capture lifecycle assertions failed')
    record_path.write_text(json.dumps(record,ensure_ascii=False,indent=2),encoding='utf-8')
finally:
    for helper in helpers:
        if helper.poll() is None:helper.terminate();helper.wait(timeout=3)
    subprocess.Popen=original_popen
    if capture_target.poll() is None:capture_target.terminate();capture_target.wait(timeout=3)
    if process is not None:
        try:process.wait(timeout=5)
        except subprocess.TimeoutExpired:process.terminate();process.wait(timeout=5)
    server.shutdown();server.server_close();thread.join()
    if 'log' in locals() and not log.closed: log.close()
    try:shutil.rmtree(temporary)
    except OSError:
        quoted=str(temporary).replace("'","''")
        cleanup=subprocess.run(["powershell","-NoProfile","-Command",f"Remove-Item -LiteralPath '{quoted}' -Recurse -Force"],capture_output=True)
        if cleanup.returncode:print("Browser scratch retained: "+str(temporary),flush=True)
