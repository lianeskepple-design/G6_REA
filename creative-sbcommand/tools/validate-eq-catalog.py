"""Validate all catalogued curves against real Rust WAV output and browser math."""
import csv
import hashlib
import json
from pathlib import Path
import re
import subprocess
import tempfile

import numpy as np
import soundfile as sf
from scipy import signal

ROOT=Path(__file__).resolve().parent.parent
OUT=ROOT/"tools/eq-curves"
data=json.loads((OUT/"eq-catalog-data.json").read_text(encoding="utf-8"))
manifest=json.loads((OUT/"manifest.json").read_text(encoding="utf-8"))
exe=ROOT/"app/target/debug/sbc-eq.exe"


def sos(setting,sr):
    rows=[]
    for fc,db in zip(setting["freqs"],setting["values"]):
        if db==0:continue
        a=10**(db/40);w=2*np.pi*fc/sr;alpha=np.sin(w)/2;c=np.cos(w);a0=1+alpha/a
        rows.append([(1+alpha*a)/a0,-2*c/a0,(1-alpha*a)/a0,1,-2*c/a0,(1-alpha/a)/a0])
    return np.asarray(rows) if rows else np.array([[1,0,0,1,0,0]])


def theoretical(setting,freqs,sr):
    _,h=signal.sosfreqz(sos(setting,sr),worN=freqs,fs=sr)
    return 20*np.log10(np.maximum(np.abs(h),1e-100))+setting["preamp"]


assert hashlib.sha256((ROOT/"reference/g6-eq-presets.json").read_bytes()).hexdigest()==manifest["source_sha256"]
assert hashlib.sha256((ROOT/"app/src/bin/sbc-eq.rs").read_bytes()).hexdigest()==manifest["software_source_sha256"]
with (OUT/"eq-modes-and-gains.csv").open(encoding="utf-8-sig",newline="") as f:rows=list(csv.DictReader(f))
assert len(rows)==68
for row in rows:
    s=data["presets"][row["preset_id"]]["settings"][row["output_mode"]]
    assert float(row["preamp_db"])==s["preamp"]
    for fc,db in zip(s["freqs"],s["values"]):assert float(row[f"gain_{fc}Hz_db"])==db
counts={}
with (OUT/"eq-frequency-response-48k.csv").open(encoding="utf-8-sig",newline="") as f:
    for row in csv.DictReader(f):
        key=(row["preset_id"],row["output_mode"]);counts[key]=counts.get(key,0)+1
        assert 20<=float(row["frequency_hz"])<=20000 and np.isfinite(float(row["response_db"]))
assert len(counts)==68 and set(counts.values())=={1024}
page_match=re.search(rb"/Type /Pages[\s\S]*?/Count (\d+)",(OUT/"all-modes-catalog.pdf").read_bytes())
assert page_match and int(page_match.group(1))==34
n,sr=131072,48000
bins=np.unique(np.rint(np.geomspace(20,20000,1024)*n/sr).astype(int))
freqs=bins*sr/n
results=[]
with tempfile.TemporaryDirectory(prefix=".eq-catalog-verify-",dir=ROOT/"tools") as temp:
    temp=Path(temp);source=temp/"impulse.wav";x=np.zeros(n,np.float32);x[0]=.25;sf.write(source,x,sr,subtype="FLOAT")
    for name in data["order"]:
        for mode,flag in [("Speaker","speakers"),("Headphone","headphones")]:
            target=temp/"output.wav"
            subprocess.run([str(exe),"--process-wav",str(source),str(target),"--eq-preset",name,"--output-mode",flag],check=True,capture_output=True)
            y,rate=sf.read(target);assert rate==sr and len(y)==n and np.isfinite(y).all()
            actual=20*np.log10(np.maximum(np.abs(np.fft.rfft(y)[bins]/.25),1e-100))
            reference=theoretical(data["presets"][name]["settings"][mode],freqs,sr)
            error=float(np.max(np.abs(actual-reference)));assert error<.001,(name,mode,error)
            results.append({"preset":name,"mode":mode,"max_abs_response_error_db":error})
            target.unlink()
html=(OUT/"index.html").read_text(encoding="utf-8")
function=html[html.index("function response("):html.index("function options(")]
cases=[];expected=[]
for name in data["order"]:
    for mode in ["Speaker","Headphone"]:
        setting=data["presets"][name]["settings"][mode]
        for rate in [44100,48000,96000]:
            frequencies=np.array([20,31,62,125,250,500,1000,2000,4000,8000,16000,20000],float)
            values=theoretical(setting,frequencies,rate)
            for frequency,value in zip(frequencies,values):
                cases.append([setting,float(frequency),rate,0]);expected.append(float(value))
node=Path("C:/Program Files/nodejs/node.exe")
program="const fs=require('fs');const a=JSON.parse(fs.readFileSync(0,'utf8'));eval(a.fn);console.log(JSON.stringify(a.cases.map(c=>response(...c))));"
proc=subprocess.run([str(node),"-e",program],input=json.dumps({"fn":function,"cases":cases}),capture_output=True,text=True,check=True)
js=np.asarray(json.loads(proc.stdout));js_error=float(np.max(np.abs(js-np.asarray(expected))))
assert js_error<1e-6,js_error
result={"date":"2026-11-12","presets":34,"mode_curves":68,"factory_csv_rows":len(rows),"response_csv_rows":sum(counts.values()),"pdf_pages":34,"rust_binary_sha256":hashlib.sha256(exe.read_bytes()).hexdigest(),"impulse_frames":n,"sample_rate":sr,"max_rust_fft_response_error_db":max(v["max_abs_response_error_db"] for v in results),"browser_math_cases":len(cases),"max_browser_vs_scipy_error_db":js_error,"checks":results,"result":"PASS","boundary":"Validates current software EQ frequency responses only; no sound card measurement."}
(OUT/"validation.json").write_text(json.dumps(result,ensure_ascii=False,indent=2),encoding="utf-8")
print(json.dumps({k:v for k,v in result.items() if k!="checks"},ensure_ascii=False,indent=2))
