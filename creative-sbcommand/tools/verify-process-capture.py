"""Verify only task-created silent targets; never capture existing user applications."""
from pathlib import Path
import base64
from contextlib import contextmanager
import uuid
import shutil
import importlib.util
import json
import os
import queue
import select
import socket
import struct
import subprocess
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.parse
import urllib.request

ROOT=Path(__file__).resolve().parent.parent
OUT=ROOT/'tools/eq-curves/audio/process-validation.json'
spec=importlib.util.spec_from_file_location('eq_capture_test_server',ROOT/'tools/serve-eq-lab.py')
module=importlib.util.module_from_spec(spec);spec.loader.exec_module(module)
module.Handler.log_message=lambda *args:None
backend=module._process_audio
native=backend.HELPER
checks=[]
def check(name,condition,**details):
    checks.append({'name':name,'passed':bool(condition),**details})
    if not condition:raise AssertionError(name)

@contextmanager
def fixture_directory():
    path=ROOT/'tools'/('.eq-idle-helper-'+uuid.uuid4().hex[:12]);path.mkdir()
    try:yield path
    finally:
        try:shutil.rmtree(path)
        except OSError:
            quoted=str(path).replace("'","''")
            subprocess.run(['powershell','-NoProfile','-Command',f"Remove-Item -LiteralPath '{quoted}' -Recurse -Force"],capture_output=True)

class Client:
    def __init__(self,port,pid,token,origin=None):
        self.socket=socket.create_connection(('127.0.0.1',port),timeout=3)
        self.socket.settimeout(3);self.buffer=b''
        origin=origin or f'http://127.0.0.1:{port}'
        path='/api/process-audio?'+urllib.parse.urlencode({'pid':pid,'token':token})
        key=base64.b64encode(os.urandom(16)).decode()
        request=f'GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nOrigin: {origin}\r\nUpgrade: websocket\r\nConnection: Upgrade\r\nSec-WebSocket-Version: 13\r\nSec-WebSocket-Key: {key}\r\n\r\n'
        self.socket.sendall(request.encode('ascii'))
        while b'\r\n\r\n' not in self.buffer:self.buffer+=self.socket.recv(4096)
        headers,self.buffer=self.buffer.split(b'\r\n\r\n',1)
        self.status=headers.split(b'\r\n',1)[0]
        if b' 101 ' not in self.status:self.socket.close()
    def exact(self,n):
        while len(self.buffer)<n:
            part=self.socket.recv(max(4096,n-len(self.buffer)))
            if not part:raise EOFError()
            self.buffer+=part
        value,self.buffer=self.buffer[:n],self.buffer[n:];return value
    def receive(self):
        h=self.exact(2);n=h[1]&127
        if n==126:n=struct.unpack('!H',self.exact(2))[0]
        elif n==127:n=struct.unpack('!Q',self.exact(8))[0]
        return h[0]&15,self.exact(n)
    def send(self,opcode,data=b'',fragment=False):
        mask=os.urandom(4);n=len(data)
        header=bytes([128|opcode,128|n]) if n<126 else bytes([128|opcode,128|126])+struct.pack('!H',n)
        message=header+mask+bytes(value^mask[i%4] for i,value in enumerate(data))
        if fragment:
            for a,b in [(0,1),(1,3),(3,7),(7,len(message))]:
                if a<b:self.socket.sendall(message[a:b]);time.sleep(.005)
        else:self.socket.sendall(message)
    def close(self):
        try:self.send(8,struct.pack('!H',1000))
        except OSError:pass
        self.socket.close()

def get_json(url,headers=None):
    with urllib.request.urlopen(urllib.request.Request(url,headers=headers or {}),timeout=6) as r:return json.load(r)

server=None;target=None;clients=[];launched=[];original_popen=subprocess.Popen
try:
    probe=json.loads(subprocess.check_output([str(native),'--probe'],creationflags=backend.CREATE_FLAGS).decode('utf-8-sig'))
    check('Windows process-loopback build support',probe.get('process_loopback_supported') is True,build=probe.get('build'))
    target=original_popen([getattr(sys,'_base_executable',sys.executable),'-c','import time; time.sleep(30)','eq-owned-silent-target'],creationflags=backend.CREATE_FLAGS)
    def tracked(*args,**kwargs):
        command=args[0] if args else kwargs.get('args',[])
        helper_call=isinstance(command,(list,tuple)) and len(command)>1 and str(command[0])==str(backend.HELPER) and command[1]=='--pid'
        if helper_call and backend.HELPER.suffix=='.py':
            command=[sys.executable,*command]
            if args:args=(command,*args[1:])
            else:kwargs['args']=command
        process=original_popen(*args,**kwargs)
        if helper_call:launched.append(process)
        return process
    subprocess.Popen=tracked
    server=module.create_server(0);thread=threading.Thread(target=server.serve_forever,daemon=True);thread.start()
    port=server.server_port;base=f'http://127.0.0.1:{port}'
    session=get_json(base+'/api/session');token=session['token']
    check('helper capability probe is available',session['capabilities']['available'] and session['capabilities']['supported'])
    data=get_json(base+'/api/processes');check('generic process list includes own background target',any(p['pid']==target.pid for p in data['processes']))
    try:get_json(base+'/api/session',{'Host':f'not-local.invalid:{port}'})
    except urllib.error.HTTPError as e:check('nonlocal Host rejected',e.code==403)
    before=len(launched)
    client=Client(port,target.pid,'invalid');check('invalid token rejected before launch',b' 403 ' in client.status and len(launched)==before)
    client=Client(port,target.pid,token,'https://other.invalid');check('foreign Origin rejected before launch',b' 403 ' in client.status and len(launched)==before)
    client=Client(port,target.pid,token);clients.append(client);check('WebSocket HTTP/1.1 upgrade',client.status==b'HTTP/1.1 101 Switching Protocols')
    ready=None;deadline=time.monotonic()+12
    while ready is None and time.monotonic()<deadline:
        opcode,data=client.receive()
        if opcode==1:
            item=json.loads(data)
            if item.get('event')=='error':raise RuntimeError('native helper error: '+item.get('message','')+' '+item.get('hresult',''))
            if item.get('event')=='ready':ready=item
        elif opcode==2:raise AssertionError('binary PCM arrived before ready metadata')
    check('real native activation ready',ready is not None and ready['channels']==2 and ready['sample_format']=='f32le',rate=ready['sample_rate'] if ready else None)
    client.send(9,b'check',fragment=True)
    started=time.monotonic();byte_count=0;pong=False
    while time.monotonic()-started<1.3:
        opcode,data=client.receive()
        if opcode==2:
            check_aligned=len(data)%8==0
            if not check_aligned:raise AssertionError('PCM alignment lost')
            if any(data):raise AssertionError('Task-created silent target unexpectedly produced nonzero audio')
            byte_count+=len(data)
        elif opcode==10 and data==b'check':pong=True
        elif opcode==1 and json.loads(data).get('event')=='error':raise RuntimeError('capture stream error')
    elapsed=time.monotonic()-started;expected=ready['sample_rate']*8
    check('real PCM throughput and complete stereo frames',byte_count/elapsed>expected*.75,bytes=byte_count,seconds=elapsed,bytes_per_second=byte_count/elapsed)
    check('fragmented client ping parsed without corruption',pong)
    helper=launched[-1];client.close();clients.remove(client);helper.wait(timeout=3)
    check('client close stops helper without stopping target',helper.poll() is not None and target.poll() is None)
    with fixture_directory() as temp:
        temp=Path(temp);source=temp/'idle_fixture.py'
        source.write_text('import sys,json; print(json.dumps({'+repr('event')+':'+repr('ready')+','+repr('sample_rate')+':48000,'+repr('channels')+':2,'+repr('sample_format')+':'+repr('f32le')+'}),file=sys.stderr,flush=True); sys.stdin.readline()',encoding='utf-8')
        backend.HELPER=source
        client=Client(port,12345,token);clients.append(client)
        opcode,data=client.receive();check('idle fixture ready',opcode==1 and json.loads(data).get('event')=='ready')
        idle_helper=launched[-1];time.sleep(5.6)
        check('no-packet interval does not abort capture',idle_helper.poll() is None)
        client.close();clients.remove(client);idle_helper.wait(timeout=3)
        check('close works even when stdout has no PCM',idle_helper.poll() is not None)
    result={'result':'PASS','checks':checks,'scope':'Only a task-created silent Python target and a temporary no-audio fixture; no existing user application was captured.'}
except Exception as error:
    result={'result':'FAIL','error':str(error),'checks':checks}
finally:
    for client in clients:
        try:client.close()
        except OSError:pass
    for process in launched:
        if process.poll() is None:process.terminate();process.wait(timeout=3)
    subprocess.Popen=original_popen;backend.HELPER=native
    if target and target.poll() is None:target.terminate();target.wait(timeout=3)
    if server:server.shutdown();server.server_close();thread.join()
OUT.write_text(json.dumps(result,ensure_ascii=False,indent=2),encoding='utf-8')
print(json.dumps(result,ensure_ascii=False,indent=2))
raise SystemExit(0 if result['result']=='PASS' else 1)
