"""Bounded G6 USB capture. Isochronous audio is discarded in memory, never saved."""
from pathlib import Path
import argparse
from collections import Counter
import json
import struct
import subprocess
import threading
import time

ROOT=Path(__file__).resolve().parent.parent
EXE='C:/Program Files/USBPcap/USBPcapCMD.exe'

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--directory',type=Path,default=ROOT/'capture/g6-usb-20261112')
    parser.add_argument('--name',default='sbx-master-01')
    parser.add_argument('--duration',type=float,default=90)
    parser.add_argument('--mark')
    parser.add_argument('--check-target',action='store_true',help='Verify the case-specific USBPcap3/address4 mapping without capture')
    args=parser.parse_args();args.directory.mkdir(parents=True,exist_ok=True)
    if args.mark:
        with (args.directory/'timeline.jsonl').open('a',encoding='utf-8') as stream:stream.write(json.dumps({'time':time.time(),'event':args.mark})+'\n')
        print('Marked '+args.mark);return
    if not 1<=args.duration<=300:parser.error('duration must be 1..300 seconds')
    output=args.directory/(args.name+'.pcap');summary=args.directory/(args.name+'.json')
    if output.exists():raise SystemExit('Refusing existing capture: '+str(output))
    interface=chr(92)*2+'.'+chr(92)+'USBPcap3'
    metadata=subprocess.run([EXE,'--extcap-interface',interface,'--extcap-config'],capture_output=True,timeout=8)
    lines=metadata.stdout.decode('utf-8',errors='replace').splitlines()
    matching=[line for line in lines if '{display=Sound BlasterX G6}' in line and '{parent=4}' in line]
    if metadata.returncode!=0 or len(matching)!=1:
        raise SystemExit('G6 is no longer confirmed at USBPcap3/address4. Re-enumerate the topology; capture refused.')
    if args.check_target:
        print(json.dumps({'confirmed':True,'interface':interface,'device_address':4},ensure_ascii=False));return
    command=[EXE,'-d',interface,'--devices','4','-s','65535','-o','-']
    stats={'state':'starting','interface':interface,'device_address':4,'kept_transfer_types':[1,2], 'started':time.time(),'command':command,'kept':0,'discarded':{},'notes':'Isochronous PCM, bulk and other device records are not written.'}
    summary.write_text(json.dumps(stats,indent=2),encoding='utf-8')
    process=subprocess.Popen(command,stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.PIPE,bufsize=0,creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
    errors=[];stop=threading.Event();counts=Counter()
    def diagnostics():
        for line in iter(process.stderr.readline,b''):
            errors.append(line.decode('utf-8',errors='replace').rstrip())
            if len(errors)>20:errors.pop(0)
    threading.Thread(target=diagnostics,daemon=True).start()
    def watchdog():
        if not stop.wait(args.duration):
            try:process.terminate()
            except OSError:pass
    threading.Thread(target=watchdog,daemon=True).start()
    def exact(n):
        result=bytearray()
        while len(result)<n:
            part=process.stdout.read(n-len(result))
            if not part:raise EOFError()
            result.extend(part)
        return bytes(result)
    try:
        header=exact(24)
        if header[:4] in (bytes.fromhex('d4c3b2a1'),bytes.fromhex('4d3cb2a1')):endian='<'
        elif header[:4] in (bytes.fromhex('a1b2c3d4'),bytes.fromhex('a1b23c4d')):endian='>'
        else:raise ValueError('USBPcap did not output a classic PCAP header: '+header[:8].hex())
        network=struct.unpack_from(endian+'I',header,20)[0]
        if network!=249:raise ValueError('Unexpected capture link type: '+str(network))
        with output.open('xb') as stream:
            stream.write(header);stream.flush();stats['state']='capturing';summary.write_text(json.dumps(stats,indent=2),encoding='utf-8')
            print(json.dumps({'ready':True,'output':str(output),'time':time.time()}),flush=True)
            while True:
                record=exact(16);captured=struct.unpack_from(endian+'I',record,8)[0]
                if captured>1048576:raise ValueError('PCAP record exceeds bounded size')
                packet=exact(captured)
                if len(packet)<27:counts['short_header']+=1;continue
                header_length=struct.unpack_from('<H',packet)[0]
                if header_length<27 or header_length>len(packet):counts['bad_usb_header']+=1;continue
                address=struct.unpack_from('<H',packet,19)[0];transfer=packet[22]
                if address!=4:counts['other_device']+=1;continue
                if transfer not in (1,2):counts['transfer_'+str(transfer)]+=1;continue
                stream.write(record);stream.write(packet);stats['kept']+=1
    except EOFError:
        stats['state']='complete' if output.exists() else 'failed'
    except Exception as error:stats['state']='failed';stats['error']=str(error)
    finally:
        stop.set()
        if process.poll() is None:process.terminate()
        try:process.wait(timeout=3)
        except subprocess.TimeoutExpired:process.kill();process.wait(timeout=2)
        stats['finished']=time.time();stats['discarded']=dict(counts);stats['exit_code']=process.returncode;stats['stderr']=errors
        summary.write_text(json.dumps(stats,ensure_ascii=False,indent=2),encoding='utf-8')
        print(json.dumps(stats,ensure_ascii=False),flush=True)
    if stats['state']=='failed':raise SystemExit(1)

if __name__=='__main__':main()
