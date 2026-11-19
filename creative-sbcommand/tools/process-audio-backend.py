"""Authenticated loopback process-audio API; stdout is read-only PCM from one helper."""
import base64
import binascii
import hashlib
import json
import os
from pathlib import Path
import queue
import secrets
import socket
import struct
import subprocess
import threading
import time

HELPER=Path(__file__).resolve().parent/'native-process-loopback'/'process-loopback.exe'
MAX_FRAME=4096
CREATE_FLAGS=getattr(subprocess,'CREATE_NO_WINDOW',0)


def session_token():return secrets.token_urlsafe(32)
def origin_ok(value,port):return value in (f'http://127.0.0.1:{port}',f'http://localhost:{port}')

def list_processes():
    if not HELPER.is_file():return {'available':False,'reason':'原生 helper 未构建/不可用','processes':[]}
    try:
        process=subprocess.run([str(HELPER),'--list'],capture_output=True,timeout=5,check=True,creationflags=CREATE_FLAGS)
        data=json.loads(process.stdout.decode('utf-8-sig'))
        items=[{'pid':int(item['pid']),'name':str(item['name'])[:200],'has_window':bool(item.get('has_window'))} for item in data if isinstance(item,dict) and 0<int(item.get('pid',0))<=0xffffffff]
        items.sort(key=lambda item:(not item['has_window'],item['name'].lower(),item['pid']))
        return {'available':True,'processes':items}
    except Exception as error:return {'available':False,'reason':f'无法读取进程列表：{error}','processes':[]}


def probe():
    if not HELPER.is_file():return {'available':False,'reason':'原生 helper 未构建/不可用'}
    try:
        process=subprocess.run([str(HELPER),'--probe'],capture_output=True,timeout=5,check=True,creationflags=CREATE_FLAGS)
        data=json.loads(process.stdout.decode('utf-8-sig'));data['available']=True
        data['supported']=bool(data.get('supported',data.get('process_loopback_supported',False)))
        if not data['supported']:data['reason']='此Windows版本不支持进程回环，要求build20348或更新版本。'
        return data
    except Exception as error:return {'available':False,'reason':f'helper 探测失败：{error}'}


def send_frame(sock,opcode,data=b''):
    count=len(data)
    length=bytes([count]) if count<126 else b'\x7e'+struct.pack('!H',count) if count<65536 else b'\x7f'+struct.pack('!Q',count)
    sock.sendall(bytes([0x80|opcode])+length+data)


def read_frame(sock,stop=None):
    def exact(count):
        data=bytearray()
        while len(data)<count:
            if stop is not None and stop.is_set():raise EOFError()
            try:part=sock.recv(count-len(data))
            except socket.timeout:continue
            if not part:raise EOFError()
            data.extend(part)
        return bytes(data)
    header=exact(2);opcode=header[0]&15
    if not header[0]&128 or header[0]&112 or not header[1]&128:raise ValueError('需要完整、masked且无扩展位的客户端帧')
    count=header[1]&127
    if count==126:count=struct.unpack('!H',exact(2))[0]
    elif count==127:count=struct.unpack('!Q',exact(8))[0]
    if count>MAX_FRAME or (opcode>=8 and count>125):raise ValueError('WebSocket控制帧过长')
    if opcode==8 and count==1:raise ValueError('无效的close帧')
    mask=exact(4);data=exact(count)
    return opcode,bytes(value^mask[index%4] for index,value in enumerate(data))


def upgrade_and_stream(handler,pid,token,expected_token):
    if not secrets.compare_digest(str(token),str(expected_token)) or not origin_ok(handler.headers.get('Origin',''),handler.server.server_port):
        handler.send_error(403,'Invalid session token or Origin');return
    if not isinstance(pid,int) or not 1<=pid<=0xffffffff:handler.send_error(400,'Invalid pid');return
    if not HELPER.is_file():handler.send_error(503,'Native helper is not built');return
    if handler.headers.get('Sec-WebSocket-Version')!='13' or handler.headers.get('Upgrade','').lower()!='websocket' or 'upgrade' not in handler.headers.get('Connection','').lower():
        handler.send_error(400,'WebSocket upgrade required');return
    key=handler.headers.get('Sec-WebSocket-Key','')
    try:
        if len(base64.b64decode(key,validate=True))!=16:raise ValueError()
    except (ValueError,binascii.Error):handler.send_error(400,'Invalid WebSocket key');return
    accept=base64.b64encode(hashlib.sha1((key+'258EAFA5-E914-47DA-95CA-C5AB0DC85B11').encode('ascii')).digest()).decode('ascii')
    handler.protocol_version='HTTP/1.1';handler.close_connection=True
    handler.send_response(101,'Switching Protocols');handler.send_header('Upgrade','websocket');handler.send_header('Connection','Upgrade');handler.send_header('Sec-WebSocket-Accept',accept);handler.end_headers()
    sock=handler.connection;sock.settimeout(.5)
    process=None;pcm=queue.Queue(maxsize=24);events=queue.Queue(maxsize=256);controls=queue.Queue();stop=threading.Event();stdout_done=threading.Event();stderr_done=threading.Event();threads=[]
    dropped=0;ready=False;started=time.monotonic();last_ping=started
    def event(item):
        try:events.put(item,timeout=.1)
        except queue.Full:
            try:events.get_nowait();events.put_nowait(item)
            except queue.Empty:pass
    try:
        process=subprocess.Popen([str(HELPER),'--pid',str(pid)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,bufsize=0,creationflags=CREATE_FLAGS)
        def pump():
            nonlocal dropped
            carry=b''
            try:
                while not stop.is_set():
                    part=process.stdout.read(8192)
                    if not part:break
                    carry+=part;length=len(carry)//8*8
                    if not length:continue
                    block,carry=carry[:length],carry[length:]
                    try:pcm.put_nowait(block)
                    except queue.Full:
                        try:pcm.get_nowait();dropped+=1;pcm.put_nowait(block)
                        except queue.Empty:pass
                if carry and not stop.is_set():event({'event':'error','message':'helper结束时留下不完整PCM帧'})
            except (OSError,ValueError):pass
            finally:stdout_done.set()
        def diagnostics():
            try:
                while not stop.is_set():
                    line=process.stderr.readline(16385)
                    if not line:break
                    if len(line)>16384:event({'event':'error','message':'helper诊断行过长'});break
                    try:
                        value=json.loads(line.decode('utf-8-sig'))
                        if isinstance(value,dict):event(value)
                    except (ValueError,UnicodeError):continue
            except (OSError,ValueError):pass
            finally:stderr_done.set()
        def reader():
            try:
                while not stop.is_set():
                    opcode,data=read_frame(sock,stop)
                    controls.put((opcode,data))
                    if opcode==8:stop.set();return
            except (EOFError,OSError,ValueError):stop.set()
        threads=[threading.Thread(target=fn,daemon=True) for fn in (pump,diagnostics,reader)]
        for thread in threads:thread.start()
        while not stop.is_set():
            try:
                while True:
                    opcode,data=controls.get_nowait()
                    if opcode==9:send_frame(sock,10,data)
                    elif opcode==10:pass
                    elif opcode==1 and data in (b'stop',b'terminate'):stop.set();break
                    elif opcode not in (8,):raise ValueError('不支持的客户端消息')
            except queue.Empty:pass
            if stop.is_set():break
            try:
                while True:
                    item=events.get_nowait()
                    if item.get('event')=='ready':
                        rate=item.get('sample_rate')
                        if item.get('channels')!=2 or item.get('sample_format')!='f32le' or not isinstance(rate,(int,float)) or not 8000<=rate<=384000:raise ValueError('helper报告了不支持的PCM格式')
                        ready=True
                    send_frame(sock,1,json.dumps(item,ensure_ascii=False,separators=(',',':')).encode('utf-8'))
                    if item.get('event')=='error':stop.set();break
            except queue.Empty:pass
            if stop.is_set():break
            if not ready and time.monotonic()-started>15:raise TimeoutError('原生采集初始化超时')
            if ready:
                try:
                    block=pcm.get(timeout=.02);send_frame(sock,2,block)
                    while not pcm.empty():send_frame(sock,2,pcm.get_nowait())
                except queue.Empty:pass
            else:time.sleep(.01)
            if ready and time.monotonic()-last_ping>10:
                send_frame(sock,9,b'eq');last_ping=time.monotonic()
                send_frame(sock,1,json.dumps({'event':'transport','dropped_blocks':dropped}).encode())
            if process.poll() is not None and stdout_done.is_set() and stderr_done.is_set() and events.empty() and (not ready or pcm.empty()):
                send_frame(sock,1,json.dumps({'event':'ended','exit_code':process.returncode,'message':'采集helper已退出'}).encode());break
    except (EOFError,ConnectionError,BrokenPipeError,socket.timeout):pass
    except Exception as error:
        try:send_frame(sock,1,json.dumps({'event':'error','message':str(error)},ensure_ascii=False).encode('utf-8'))
        except OSError:pass
    finally:
        stop.set()
        if process and process.poll() is None:
            try:process.stdin.write(b'stop\n');process.stdin.flush()
            except (OSError,ValueError):pass
            try:process.wait(timeout=1)
            except subprocess.TimeoutExpired:
                process.terminate()
                try:process.wait(timeout=1)
                except subprocess.TimeoutExpired:process.kill();process.wait(timeout=1)
        if process:
            for stream in (process.stdin,process.stdout,process.stderr):
                try:stream.close()
                except (OSError,ValueError):pass
        try:send_frame(sock,8,struct.pack('!H',1000))
        except OSError:pass
        for thread in threads:thread.join(timeout=.6)
