"""Render deterministic diagnostic A/B audio with the actual sbc-eq executable."""
from pathlib import Path
import hashlib
import json
import math
import subprocess
import tempfile

import numpy as np
import soundfile as sf
from scipy import signal

ROOT=Path(__file__).resolve().parent.parent
CAT=ROOT/"tools/eq-curves"
OUT=CAT/"audio"
WAVS=OUT/"wavs"
WAVS.mkdir(parents=True,exist_ok=True)
DATA=json.loads((CAT/"eq-catalog-data.json").read_text(encoding="utf-8"))
EXE=ROOT/"app/target/debug/sbc-eq.exe"
SR,DURATION,SEED=48000,16,20261112
SEGMENTS=[{"start":0,"end":4,"label":"低频脉冲与谐波"},
          {"start":4,"end":8,"label":"中频谐波与共振峰形状（非人声录音）"},
          {"start":8,"end":12,"label":"短促瞬态与摩擦噪声（非真实脚步录音）"},
          {"start":12,"end":16,"label":"宽带噪声与高频层次"}]
MODES=[("Speaker","speakers"),("Headphone","headphones")]


def fixture():
    rng=np.random.default_rng(SEED)
    x=np.zeros((SR*DURATION,2),np.float64)
    def add(start,mono,pan=0):
        start=int(round(start*SR));n=min(len(mono),len(x)-start)
        if n<=0:return
        left=math.sqrt((1-pan)/2);right=math.sqrt((1+pan)/2)
        x[start:start+n,0]+=mono[:n]*left;x[start:start+n,1]+=mono[:n]*right
    t=np.arange(SR*4)/SR
    low=.12*np.sin(2*np.pi*31*t)+.17*np.sin(2*np.pi*62*t)+.10*np.sin(2*np.pi*125*t)
    add(0,low,-.15)
    for i in range(8):
        q=np.arange(int(SR*.4))/SR
        kick=np.sin(2*np.pi*(85*q-25*q*q))*np.exp(-q*16)
        add(i*.48,.55*kick,(-1)**i*.25)
    fundamental=135+20*np.sin(2*np.pi*.45*t)
    phase=2*np.pi*np.cumsum(fundamental)/SR
    vowel=np.zeros_like(t)
    for h in range(1,68):
        hz=h*fundamental
        envelope=(np.exp(-.5*((hz-650)/160)**2)+.6*np.exp(-.5*((hz-1250)/230)**2)+.35*np.exp(-.5*((hz-2700)/420)**2))
        vowel+=np.sin(h*phase)*envelope/(h**.75)
    vowel*=.11*(.35+.65*np.sin(2*np.pi*2.8*t)**2)
    add(4,vowel,.25)
    bandpass=signal.butter(3,[250,6500],btype="bandpass",fs=SR,output="sos")
    for i in range(9):
        q=np.arange(int(SR*.28))/SR
        noise=signal.sosfilt(bandpass,rng.standard_normal(len(q)))
        transient=.34*noise*np.exp(-q*25)+.22*np.sin(2*np.pi*185*q)*np.exp(-q*30)
        add(8+i*.43,transient,(-1)**i*.55)
    noise=rng.standard_normal(len(t))
    broad=signal.sosfilt(signal.butter(2,16000,fs=SR,output="sos"),noise)*.055
    top=sum(.018*np.sin(2*np.pi*f*t+.3*j) for j,f in enumerate([2000,4000,8000,12000,16000]))
    add(12,(broad+top)*(.4+.6*np.sin(2*np.pi*.75*t)**2),-.2)
    # Fade each diagnostic segment rather than introducing boundary clicks.
    fade=int(SR*.035)
    for a in range(4):
        lo=a*4*SR;hi=(a+1)*4*SR
        x[lo:lo+fade]*=np.linspace(0,1,fade)[:,None]
        x[hi-fade:hi]*=np.linspace(1,0,fade)[:,None]
    x*=.72/np.max(np.abs(x))
    return x.astype(np.float32)


def signature(s):
    return json.dumps([float(s["preamp"]),list(map(float,s["freqs"])),list(map(float,s["values"]))],separators=(",",":"))


source=fixture()
groups={}
mapping={}
for name in DATA["order"]:
    mapping[name]={}
    for mode,flag in MODES:
        s=DATA["presets"][name]["settings"][mode];key=signature(s)
        if key not in groups:
            is_flat=s["preamp"]==0 and all(g==0 for g in s["values"])
            filename="original.wav" if is_flat else f"{name}_{mode}.wav"
            groups[key]={"settings":s,"name":name,"mode":mode,"flag":flag,"filename":filename,"aliases":[],"flat":is_flat}
        groups[key]["aliases"].append(f"{name}/{mode}")
        mapping[name][mode]={"path":"audio/wavs/"+groups[key]["filename"]}
rendered={}
with tempfile.TemporaryDirectory(prefix=".eq-audio-render-",dir=ROOT/"tools") as temp:
    temp=Path(temp);raw=temp/"source.wav";sf.write(raw,source,SR,subtype="FLOAT")
    maximum=float(np.max(np.abs(source)))
    for key,g in groups.items():
        if g["flat"]:audio=source.astype(np.float64)
        else:
            target=temp/"render.wav"
            subprocess.run([str(EXE),"--process-wav",str(raw),str(target),"--eq-preset",g["name"],"--output-mode",g["flag"]],capture_output=True,check=True)
            audio,rate=sf.read(target,always_2d=True)
            assert rate==SR and audio.shape==source.shape and np.isfinite(audio).all()
            target.unlink()
        rendered[key]=audio
        maximum=max(maximum,float(np.max(np.abs(audio))))
    # A single attenuation shared by every file preserves EQ-dependent loudness.
    common_gain=min(1.0,.80/maximum)
    extra_db=20*math.log10(common_gain)
    for key,g in groups.items():
        samples=rendered[key]*common_gain
        assert np.max(np.abs(samples))<=.80000001
        sf.write(WAVS/g["filename"],samples,SR,subtype="PCM_16")

original_path=WAVS/"original.wav"
original,rate=sf.read(original_path,always_2d=True)
checks=[]
for key,g in groups.items():
    path=WAVS/g["filename"];actual,rate=sf.read(path,always_2d=True)
    expected=rendered[key]*common_gain
    error=float(np.max(np.abs(actual-expected)))
    assert error<=1/32768+1e-12 and rate==SR and actual.shape==source.shape and np.max(np.abs(actual))<1
    checks.append({"path":"audio/wavs/"+g["filename"],"aliases":g["aliases"],"frames":len(actual),"peak":float(np.max(np.abs(actual))),"rms_dbfs":float(20*np.log10(np.sqrt(np.mean(actual**2))+1e-30)),"quantization_max_abs_error":error,"sha256":hashlib.sha256(path.read_bytes()).hexdigest()})
examples=[]
for preset,start,end,title in [("FootstepEnhancer",8,12,"脚步增强"),("FPS",0,4,"FPS耳机"),("Music",4,8,"音乐"),("Flat",8,12,"平直校验")]:
    b,_=sf.read(CAT/mapping[preset]["Headphone"]["path"],always_2d=True)
    a0,b0=int(start*SR),int(end*SR)
    pair=np.concatenate([original[a0:b0],np.zeros((SR//4,2)),b[a0:b0]])
    target=OUT/(preset+"-A-then-B.wav");sf.write(target,pair,SR,subtype="PCM_16")
    examples.append({"preset":preset,"label":title,"mode":"Headphone","path":"audio/"+target.name,"original_start":start,"original_end":end,"a_seconds":end-start,"silence_seconds":.25,"b_seconds":end-start})
manifest={"date":"2026-11-12","ready":True,"sample_rate":SR,"duration_seconds":DURATION,"channels":2,"encoding":"PCM_16","Q":1,"seed":SEED,"extra_gain_db":extra_db,"common_linear_gain":common_gain,"source_kind":"synthetic_diagnostic_fixture","description":"本地合成诊断素材，非真实游戏、脚步或人声录音","normalization":"Only one common attenuation is applied to A and every B; no per-preset loudness normalization.","original":{"path":"audio/wavs/original.wav"},"segments":SEGMENTS,"presets":mapping,"examples":examples,"unique_files":len(groups),"preset_mode_mappings":68,"software_binary_sha256":hashlib.sha256(EXE.read_bytes()).hexdigest(),"factory_data_sha256":hashlib.sha256((ROOT/"reference/g6-eq-presets.json").read_bytes()).hexdigest(),"checks":checks}
(OUT/"audio-manifest.json").write_text(json.dumps(manifest,ensure_ascii=False,indent=2),encoding="utf-8")
(OUT/"audio-manifest.js").write_text("window.EQ_AUDIO_DATA = "+json.dumps(manifest,ensure_ascii=False,separators=(",",":"))+";\n",encoding="utf-8")
readme=["# EQ 原音 / 效果对比音频","","生成日期：2026-11-12（UTC+8）。全部素材为固定种子合成诊断音频，无用户录音、真实游戏脚步或第三方音乐。","","## 试听入口","","打开上一级index.html，在播放器中切换原音A与EQ B；预设及耳机/扬声器模式对应原图册。播放器不会自动播放。","","- [原音 A](wavs/original.wav)：16秒、48 kHz、双声道PCM16。","- [脚步增强 A→B](FootstepEnhancer-A-then-B.wav)：前4秒原音、0.25秒静音、后4秒EQ，使用同一段瞬态素材。","- [FPS耳机 A→B](FPS-A-then-B.wav)：同格式，使用低频片段。","- [音乐 A→B](Music-A-then-B.wav)：同格式，使用中频谐波片段。","- [平直校验 A→B](Flat-A-then-B.wav)：两段相同，供校验，不应人为听出差异。","","## 16秒素材分段",""]
readme += [f"- {s['start']}–{s['end']}秒：{s['label']}。" for s in SEGMENTS]
readme += ["","## 比较口径","",f"实际使用sbc-eq生成34预设×2输出模式；33种不同曲线/基线共用音频文件。所有版本统一附加{extra_db:.6f} dB（线性{common_gain:.9f}），仅为了保留峰值余量，未逐预设归一化。A与所有B采用同一个增益，因此响度差仍属于EQ变化。", "", "浏览器试听固定48 kHz、Q=1及上述公共衰减；图册的采样率/附加增益控件只改变曲线，不会实时重新生成试听音频。PCM16仅用于通用播放，Rust中间结果为float32，量化误差不超过1/32768。", "", "68个预设/模式映射及每个文件的SHA256、峰值、量化误差见audio-manifest.json。相同曲线共用同一文件，Speaker/Headphone不同的预设保留不同文件。", "", "复现：在creative-sbcommand目录运行 `./tools/audio-env/Scripts/python.exe -X utf8 ./tools/build-eq-audio.py`。本脚本不打开任何音频设备。"]
(OUT/"README.md").write_text("\n".join(readme)+"\n",encoding="utf-8")
print(json.dumps({"preset_mode_mappings":68,"unique_audio_files":len(groups),"duration_seconds":DURATION,"common_gain_db":extra_db,"max_preview_peak":max(c["peak"] for c in checks),"all_paths_exist":all((CAT/v["path"]).is_file() for p in mapping.values() for v in p.values()),"output_bytes":sum(p.stat().st_size for p in OUT.rglob("*.wav"))},ensure_ascii=False,indent=2))
