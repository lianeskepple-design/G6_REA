"""Serve only the EQ catalogue on loopback; no audio upload endpoint exists."""
from functools import partial
from http.server import SimpleHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
import argparse
import hashlib
import json
import re
import urllib.request
import webbrowser
import sys
import urllib.parse
import importlib.util

_spec=importlib.util.spec_from_file_location("process_audio_backend",Path(__file__).with_name("process-audio-backend.py"))
_process_audio=importlib.util.module_from_spec(_spec);_spec.loader.exec_module(_process_audio)

ROOT=Path(__file__).resolve().parent.parent
SITE=ROOT/"tools/eq-curves"
_server_sources=[Path(__file__),Path(__file__).with_name('process-audio-backend.py')]
SITE_ID=hashlib.sha256(str(SITE.resolve()).encode('utf-8')+b''.join(path.read_bytes() for path in _server_sources)).hexdigest()[:16]

class Handler(SimpleHTTPRequestHandler):
    def log_message(self,format,*args):
        safe=tuple(re.sub(r'([?&]token=)[^&\s]+',r'\1[redacted]',value) if isinstance(value,str) else value for value in args)
        super().log_message(format,*safe)
    def partial_file(self,head=False):
        requested=self.headers.get('Range')
        path=Path(self.translate_path(self.path))
        if not requested or not path.is_file():return False
        total=path.stat().st_size
        match=re.fullmatch(r'bytes=(\d*)-(\d*)',requested.strip())
        try:
            if not match or not any(match.groups()) or total==0:raise ValueError()
            first,last=match.groups()
            if first:start=int(first);end=min(int(last) if last else total-1,total-1)
            else:start=max(0,total-int(last));end=total-1
            if start<0 or start>=total or end<start:raise ValueError()
        except ValueError:
            self.send_response(416);self.send_header('Content-Range',f'bytes */{total}');self.send_header('Content-Length','0');self.end_headers();return True
        self.send_response(206);self.send_header('Content-Type',self.guess_type(str(path)))
        self.send_header('Content-Range',f'bytes {start}-{end}/{total}');self.send_header('Content-Length',str(end-start+1));self.end_headers()
        if not head:
            with path.open('rb') as source:
                source.seek(start);remaining=end-start+1
                while remaining:
                    chunk=source.read(min(65536,remaining))
                    if not chunk:break
                    self.wfile.write(chunk);remaining-=len(chunk)
        return True
    def do_HEAD(self):
        if not self.partial_file(head=True):super().do_HEAD()
    def json_response(self,payload,status=200):
        body=json.dumps(payload,ensure_ascii=False).encode('utf-8')
        self.send_response(status);self.send_header('Content-Type','application/json; charset=utf-8');self.send_header('Content-Length',str(len(body)));self.end_headers();self.wfile.write(body)
    def do_GET(self):
        parsed=urllib.parse.urlsplit(self.path)
        allowed_hosts=(f'127.0.0.1:{self.server.server_port}',f'localhost:{self.server.server_port}')
        if parsed.path.startswith('/api/') and self.headers.get('Host','') not in allowed_hosts:
            self.send_error(403,'Invalid local Host');return
        if parsed.path=="/api/session":
            self.json_response({'token':self.server.eq_token,'capabilities':_process_audio.probe()})
        elif parsed.path=="/api/processes":
            self.json_response(_process_audio.list_processes())
        elif parsed.path=="/api/process-audio":
            query=urllib.parse.parse_qs(parsed.query)
            try: pid=int(query.get('pid',[''])[0])
            except ValueError: self.send_error(400,'Invalid pid');return
            if not 1<=pid<=2**31-1:self.send_error(400,'Invalid pid');return
            _process_audio.upgrade_and_stream(self,pid,query.get('token',[''])[0],self.server.eq_token)
        elif parsed.path=="/__eq_lab_health":
            payload=json.dumps({"app":"creative-eq-lab","site_id":SITE_ID}).encode("utf-8")
            self.send_response(200);self.send_header("Content-Type","application/json");self.send_header("Content-Length",str(len(payload)));self.end_headers();self.wfile.write(payload)
        else:
            try:
                if not self.partial_file():super().do_GET()
            except (BrokenPipeError,ConnectionAbortedError,ConnectionResetError):pass
    def do_POST(self):
        self.send_error(405,"Audio uploads are not supported. Decode selected files locally in the browser.")
    def end_headers(self):
        self.send_header("Accept-Ranges","bytes")
        self.send_header("Cache-Control","no-cache")
        self.send_header("Permissions-Policy","microphone=(self), speaker-selection=(self), display-capture=(self)")
        self.send_header('X-Frame-Options','SAMEORIGIN')
        super().end_headers()


def create_server(port=0):
    server=ThreadingHTTPServer(("127.0.0.1",port),partial(Handler,directory=str(SITE)))
    server.eq_token=_process_audio.session_token()
    return server


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--port",type=int,default=8767)
    parser.add_argument("--no-browser",action="store_true")
    args=parser.parse_args()
    if not 0<=args.port<=65535:parser.error("port must be between 0 and 65535")
    try:server=create_server(args.port)
    except OSError:
        try:
            with urllib.request.urlopen(f"http://127.0.0.1:{args.port}/__eq_lab_health",timeout=1) as response:
                health=json.load(response)
            if health.get("app")=="creative-eq-lab" and health.get("site_id")==SITE_ID:
                url=f"http://127.0.0.1:{args.port}/audio-lab.html"
                print("Existing EQ lab: "+url,flush=True)
                if not args.no_browser:webbrowser.open(url)
                return
        except Exception:pass
        server=create_server(0)
    url=f"http://127.0.0.1:{server.server_port}/audio-lab.html"
    print("EQ audio lab: "+url,flush=True)
    print("Loopback only. Audio files are decoded locally, not uploaded. Ctrl+C stops the server.",flush=True)
    if not args.no_browser:webbrowser.open(url)
    try:server.serve_forever()
    except KeyboardInterrupt:pass
    finally:server.server_close()

if __name__=="__main__":main()
