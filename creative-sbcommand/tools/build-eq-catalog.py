"""Build an exportable G6 EQ catalogue from factory data and the software EQ model."""
from pathlib import Path
import csv
import hashlib
import json
import math
import os
import tempfile

ROOT = Path(__file__).resolve().parent.parent
OUT = ROOT / "tools/eq-curves"
OUT.mkdir(exist_ok=True)
cache = tempfile.TemporaryDirectory(prefix=".eq-plot-", dir=ROOT / "tools")
os.environ["MPLCONFIGDIR"] = cache.name
import numpy as np
from scipy import signal
import matplotlib
matplotlib.use("Agg")
import matplotlib.pyplot as plt
from matplotlib import font_manager
from matplotlib.backends.backend_pdf import PdfPages

font = Path("C:/Windows/Fonts/msyh.ttc")
if font.exists():
    font_manager.fontManager.addfont(str(font))
    plt.rcParams["font.family"] = font_manager.FontProperties(fname=str(font)).get_name()
plt.rcParams["axes.unicode_minus"] = False
plt.rcParams["font.size"] = 10
SOURCE = ROOT / "reference/g6-eq-presets.json"
CODE = ROOT / "app/src/bin/sbc-eq.rs"
presets = json.loads(SOURCE.read_text(encoding="utf-8"))["eqPresets"]
assert len(presets) == 34
LABELS = {
    "Acoustic":"原声", "AdventureAndAction":"动作与冒险", "ApexLegend":"Apex 英雄",
    "ArenaOfValor":"传说对决", "CallofDutySeries":"使命召唤系列", "Classical":"古典",
    "CounterStrikeGlobalOffensive":"CS:GO", "Country":"乡村", "Dance":"舞曲", "Dota2":"Dota 2",
    "DrivingSimulation":"驾驶模拟", "Flat":"平直", "FootstepEnhancer":"脚步增强", "Fortnite":"堡垒之夜",
    "FPS":"第一人称射击", "Gaming":"游戏通用", "HipHop":"嘻哈", "Jazz":"爵士",
    "LeagueOfLegends":"英雄联盟", "MetalGearSolidVThePhantomPain":"合金装备 V：幻痛",
    "Movie":"电影", "Music":"音乐", "Overwatch":"守望先锋", "PlayerUnknown":"PUBG / 绝地求生",
    "Pop":"流行", "ProjectCars":"赛车计划", "RealTimeStrategy":"即时战略", "RnB":"R&B",
    "Rock":"摇滚", "RocketLeague":"火箭联盟", "RPG":"角色扮演", "Stadium":"体育场",
    "TheWitcher3WildHunt":"巫师 3：狂猎", "Vocal":"人声",
}
MUSIC = {"Acoustic","Classical","Country","Dance","HipHop","Jazz","Pop","RnB","Rock","Vocal"}
GENERAL = {"Flat","Music","Movie","FootstepEnhancer"}
ORDER = sorted(presets, key=lambda k:(presets[k]["order"], k))
SR, HEADROOM, Q = 48000, 0.0, 1.0
FREQS = np.geomspace(20, 20000, 1024)
MODES = [("Speaker", "扬声器", "#147d92"), ("Headphone", "耳机", "#df7c20")]


def category(name):
    return "音乐风格" if name in MUSIC else "通用与增强" if name in GENERAL else "游戏与场景"


def sos_for(setting, sr=SR):
    rows = []
    for fc, db in zip(setting["freqs"], setting["values"]):
        if db == 0:
            continue
        if fc >= sr / 2:
            raise ValueError("active band exceeds Nyquist")
        a = 10 ** (db / 40)
        omega = 2 * np.pi * fc / sr
        alpha = np.sin(omega) / (2 * Q)
        cosine = np.cos(omega)
        a0 = 1 + alpha / a
        rows.append([(1+alpha*a)/a0, -2*cosine/a0, (1-alpha*a)/a0,
                     1, -2*cosine/a0, (1-alpha/a)/a0])
    return np.asarray(rows) if rows else np.asarray([[1, 0, 0, 1, 0, 0]])


def response(setting, frequencies=FREQS, sr=SR):
    _, h = signal.sosfreqz(sos_for(setting, sr), worN=frequencies, fs=sr)
    return 20 * np.log10(np.maximum(np.abs(h), 1e-100)) + setting["preamp"] + HEADROOM


curves = {(name, mode):response(presets[name]["settings"][mode]) for name in ORDER for mode,_,_ in MODES}
assert all(np.isfinite(v).all() for v in curves.values())
different = [name for name in ORDER if presets[name]["settings"]["Speaker"] != presets[name]["settings"]["Headphone"]]
assert len(different) == 18
assert np.max(np.abs(curves[("Flat", "Speaker")])) == 0
nominal_header = ["preset_id", "中文整理名", "分类", "output_mode", "原厂Type", "Order", "preamp_db", *[f"gain_{f}Hz_db" for f in presets["Flat"]["settings"]["Speaker"]["freqs"]]]
with (OUT/"eq-modes-and-gains.csv").open("w", newline="", encoding="utf-8-sig") as f:
    writer=csv.writer(f); writer.writerow(nominal_header)
    for name in ORDER:
        for mode,label,_ in MODES:
            p=presets[name];s=p["settings"][mode]
            writer.writerow([name,LABELS[name],category(name),mode,p["type"],p["order"],s["preamp"],*s["values"]])
with (OUT/"eq-frequency-response-48k.csv").open("w", newline="", encoding="utf-8-sig") as f:
    writer=csv.writer(f);writer.writerow(["preset_id","output_mode","sample_rate_hz","software_Q","extra_gain_db","frequency_hz","response_db"])
    for name in ORDER:
        for mode,_,_ in MODES:
            writer.writerows((name,mode,SR,Q,HEADROOM,format(hz,".9f"),format(db,".9f")) for hz,db in zip(FREQS,curves[(name,mode)]))
peak_rows=[]
for name in ORDER:
    for mode,label,_ in MODES:
        v=curves[(name,mode)];imax=int(np.argmax(v));imin=int(np.argmin(v))
        peak_rows.append({"preset_id":name,"mode":mode,"peak_db":float(v[imax]),"peak_hz":float(FREQS[imax]),"min_db":float(v[imin]),"min_hz":float(FREQS[imin])})
payload={"sample_rate":SR,"q":Q,"extra_gain_db":HEADROOM,"date":"2026-11-12","order":ORDER,"labels":LABELS,"categories":{n:category(n) for n in ORDER},"presets":presets}
(OUT/"eq-catalog-data.json").write_text(json.dumps(payload,ensure_ascii=False,indent=2),encoding="utf-8")
manifest={"date":"2026-11-12","source":str(SOURCE),"source_sha256":hashlib.sha256(SOURCE.read_bytes()).hexdigest(),"software_source":str(CODE),"software_source_sha256":hashlib.sha256(CODE.read_bytes()).hexdigest(),"presets":34,"output_mode_curves":68,"different_mode_pairs":18,"same_mode_pairs":16,"frequency_points_per_curve":1024,"sample_rate_hz":SR,"Q":Q,"extra_gain_db":HEADROOM,"peak_summary":peak_rows,"boundary":"Software-model theoretical response; not a hardware measurement. Chinese labels/categories are editorial."}
(OUT/"manifest.json").write_text(json.dumps(manifest,ensure_ascii=False,indent=2),encoding="utf-8")
ylo=min(-12,math.floor(min(v.min() for v in curves.values())/3)*3)
yhi=max(12,math.ceil(max(v.max() for v in curves.values())/3)*3)


def plot(ax,name,markers=False,compact=False):
    for mode,label,color in MODES:
        setting=presets[name]["settings"][mode]
        ax.semilogx(FREQS,curves[(name,mode)],color=color,lw=1.4 if compact else 2,label=label+"：软件频响",linestyle="-" if mode=="Speaker" else "--")
        if markers:
            ax.scatter(setting["freqs"],setting["values"],s=27,marker="o" if mode=="Speaker" else "x",color=color,label=label+"：原厂增益点",zorder=5)
    ax.axhline(0,color="#64748b",lw=.6)
    ax.set_xlim(20,20000);ax.set_ylim(ylo,yhi);ax.grid(True,which="both",alpha=.2)
    ticks=[31,125,500,2000,8000,16000] if compact else [20,31,62,125,250,500,1000,2000,4000,8000,16000,20000]
    ax.set_xticks(ticks,[str(t) if t<1000 else f"{t/1000:g}k" for t in ticks])
    ax.tick_params(labelsize=7 if compact else 9)
    ax.set_title(LABELS[name]+" / "+name,fontsize=9 if compact else 14,pad=8)
    if not compact:ax.set_xlabel("频率 / Hz（对数轴）");ax.set_ylabel("增益 / dB")


fig,axes=plt.subplots(9,4,figsize=(17,24),constrained_layout=True)
for ax,name in zip(axes.flat,ORDER):plot(ax,name,compact=True)
for ax in axes.flat[len(ORDER):]:ax.axis("off")
fig.suptitle("G6 离线 EQ：34 个预设 × 扬声器 / 耳机\n48 kHz · Q=1 · 附加增益 0 dB · 青色实线：扬声器 / 橙色虚线：耳机 · 软件理论频响，非硬件实测",fontsize=15)
fig.savefig(OUT/"all-modes-overview.png",dpi=165);plt.close(fig)
with PdfPages(OUT/"all-modes-catalog.pdf") as pdf:
    for name in ORDER:
        fig=plt.figure(figsize=(11.7,8.3));ax=fig.add_axes([.085,.34,.88,.53]);plot(ax,name,markers=True);ax.legend(loc="upper right",ncol=2,fontsize=9)
        rows=[[label,*[f"{x:g}" for x in presets[name]["settings"][mode]["values"]]] for mode,label,_ in MODES]
        table=fig.add_axes([.06,.13,.9,.13]);table.axis("off")
        art=table.table(cellText=rows,colLabels=["增益 dB","31","62","125","250","500","1k","2k","4k","8k","16k"],cellLoc="center",loc="center");art.auto_set_font_size(False);art.set_fontsize(10);art.scale(1,1.6)
        fig.text(.06,.06,"48 kHz / Q=1.0 / 附加增益 0 dB。点：原厂频段增益；线：软件滤波器级联响应，包含频段叠加。\n原厂 Q、拓扑和硬件频响未证实；中文名称为整理译名。",fontsize=10)
        pdf.savefig(fig)
        if name=="FootstepEnhancer":fig.savefig(OUT/"footstep-enhancer.png",dpi=180)
        plt.close(fig)

HTML = r'''<!doctype html><html lang="zh-CN"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>G6 离线 EQ 模式与频响</title>
<style>*{box-sizing:border-box}body{margin:0;background:#f4f7fa;color:#172738;font:15px/1.6 system-ui,"Microsoft YaHei",sans-serif}header{padding:26px 30px 16px}h1{margin:0;font-size:26px}header p{color:#526477;margin:8px 0}.layout{display:grid;grid-template-columns:270px minmax(0,1fr);gap:18px;padding:0 30px 30px}aside,main{background:white;border:1px solid #dbe3ea;border-radius:14px;padding:18px}input,select,button{font:inherit;border:1px solid #becbd7;border-radius:7px;padding:8px;background:white;color:inherit}aside input,aside select{width:100%;margin-bottom:10px}aside select{height:490px;padding:4px}.controls{display:flex;flex-wrap:wrap;gap:12px;align-items:center}.controls label{font-size:13px}#plot{display:block;width:100%;margin-top:18px;touch-action:pan-y}#tip{font-family:ui-monospace,monospace;min-height:25px;color:#536579}.muted{color:#5d6f81;font-size:13px}.table{overflow:auto}table{width:100%;border-collapse:collapse;font-size:13px;margin-top:14px}th,td{padding:9px 7px;border-bottom:1px solid #e2e8ef;text-align:right;white-space:nowrap}td:first-child,th:first-child{text-align:left}a{color:#086f91}footer{margin-top:15px;border-top:1px solid #e2e8ef;padding-top:12px}.pill{display:inline-block;background:#e9f4f6;padding:2px 8px;border-radius:15px;font-size:12px}#name{font-size:21px;margin:0 0 10px}@media(max-width:800px){.layout{display:block;padding:0 12px 18px}header{padding:20px 14px}aside{margin-bottom:12px}aside select{height:150px}main{padding:12px}}</style>
<header><a href="audio-lab.html">打开本地音频 EQ 实验室 →</a><h1>G6 离线 EQ · 模式与频响图册</h1><p>34 个原厂预设 · 68 条输出模式曲线 · 18 组扬声器 / 耳机设置不同</p><p class="muted">平滑线是当前软件 EQ（十段 RBJ / Q=1）的理论响应；离散点是原厂增益设置。不是 G6 硬件实测，也不是完整 SBX / Scout 效果。</p></header>
<div class="layout"><aside><input id="search" placeholder="搜索：脚步 / FPS / 游戏名"><select id="preset" size="20" aria-label="预设"></select><div class="muted">中文名称与分类为整理译名，调用程序请用英文 ID。<br><a href="eq-modes-and-gains.csv">原厂增益 CSV</a> · <a href="eq-frequency-response-48k.csv">48k 频响 CSV</a><br><a href="all-modes-catalog.pdf">完整 PDF</a> · <a href="all-modes-overview.png">总览 PNG</a></div></aside>
<main><h2 id="name"></h2><div id="status" class="muted"></div><div class="controls"><label>输出模式 <select id="mode"><option value="both">扬声器 / 耳机对比</option><option value="Headphone">耳机</option><option value="Speaker">扬声器</option></select></label><label>采样率 <select id="sr"><option value="48000">48 kHz</option><option value="44100">44.1 kHz</option><option value="96000">96 kHz</option></select></label><label>附加增益 dB <input id="gain" type="number" value="0" min="-24" max="24" step="0.5" style="width:80px"></label><label><input id="markers" type="checkbox" checked>原厂增益点</label><button id="export">导出当前曲线 CSV</button></div>
<svg id="plot" viewBox="0 0 1000 430" role="img" aria-label="频率响应图"></svg><div id="tip">移动到曲线上查看频率和响应。</div><div class="table" id="bands"></div><footer class="muted">频率为对数轴，增益单位 dB。带符号附加增益：负数衰减、正数提升，默认 0 dB。原厂点不包含附加增益；各段叠加后，曲线在标称频点处不一定等于滑块数值。所有计算在本页面本地完成，不需要联网。</footer></main></div>
<script id="data" type="application/json">__DATA__</script><script>
const D=JSON.parse(document.getElementById('data').textContent),$=id=>document.getElementById(id),ns='http://www.w3.org/2000/svg';let current='FootstepEnhancer',shown=[],grid=[],values={},limits=[-12,12];const colors={Speaker:'#147d92',Headphone:'#df7c20'},labels={Speaker:'扬声器',Headphone:'耳机'},L=62,R=974,T=35,B=360;
function elem(tag,attrs,text){let e=document.createElementNS(ns,tag);for(let k in attrs)e.setAttribute(k,attrs[k]);if(text!==undefined)e.textContent=text;$('plot').appendChild(e);return e}
function response(s,f,sr,g){let total=s.preamp+g;let w=2*Math.PI*f/sr;for(let k=0;k<10;k++){let db=s.values[k];if(db===0)continue;let w0=2*Math.PI*s.freqs[k]/sr,A=10**(db/40),alpha=Math.sin(w0)/2,c=Math.cos(w0),a0=1+alpha/A,b0=(1+alpha*A)/a0,b1=-2*c/a0,b2=(1-alpha*A)/a0,a1=-2*c/a0,a2=(1-alpha/A)/a0;let nr=b0+b1*Math.cos(w)+b2*Math.cos(2*w),ni=-b1*Math.sin(w)-b2*Math.sin(2*w),dr=1+a1*Math.cos(w)+a2*Math.cos(2*w),di=-a1*Math.sin(w)-a2*Math.sin(2*w);total+=10*Math.log10((nr*nr+ni*ni)/(dr*dr+di*di))}return total}
function options(){let q=$('search').value.toLowerCase();$('preset').replaceChildren();for(let n of D.order){if(!(n+' '+D.labels[n]+' '+D.categories[n]).toLowerCase().includes(q))continue;let o=document.createElement('option');o.value=n;o.textContent=D.labels[n]+' · '+n;$('preset').appendChild(o)}$('preset').value=current;if(!$('preset').value&&$('preset').options.length){current=$('preset').options[0].value;$('preset').value=current;render()}}
function render(){const sr=+$('sr').value;let gain=+$('gain').value;if(!Number.isFinite(gain))gain=0;gain=Math.min(24,Math.max(-24,gain));const p=D.presets[current];shown=$('mode').value==='both'?['Speaker','Headphone']:[$('mode').value];grid=Array.from({length:1024},(_,i)=>20*1000**(i/1023));values={};for(let m of shown)values[m]=grid.map(f=>response(p.settings[m],f,sr,gain));let all=shown.flatMap(m=>values[m]);if($('markers').checked)all.push(...shown.flatMap(m=>p.settings[m].values));limits=[Math.min(-12,3*Math.floor(Math.min(...all)/3)),Math.max(12,3*Math.ceil(Math.max(...all)/3))];let x=f=>L+(Math.log10(f/20)/3)*(R-L),y=v=>B-(v-limits[0])/(limits[1]-limits[0])*(B-T);$('plot').replaceChildren();for(let f of [20,31,62,125,250,500,1000,2000,4000,8000,16000,20000]){let a=x(f);elem('line',{x1:a,y1:T,x2:a,y2:B,stroke:'#e1e7ee'});elem('text',{x:a,y:B+22,'text-anchor':'middle',fill:'#536579','font-size':11},f>=1000?f/1000+'k':f)}for(let v=limits[0];v<=limits[1];v+=3){let a=y(v);elem('line',{x1:L,y1:a,x2:R,y2:a,stroke:v===0?'#8395a7':'#e1e7ee'});elem('text',{x:L-10,y:a+4,'text-anchor':'end',fill:'#536579','font-size':12},v)}for(let m of shown){elem('path',{d:grid.map((f,i)=>(i?'L':'M')+x(f).toFixed(2)+','+y(values[m][i]).toFixed(2)).join(' '),fill:'none',stroke:colors[m],'stroke-width':2.4,'stroke-dasharray':m==='Headphone'?'7 4':'none'});if($('markers').checked)p.settings[m].freqs.forEach((f,i)=>elem('circle',{cx:x(f),cy:y(p.settings[m].values[i]),r:m==='Speaker'?4.5:3,stroke:colors[m],fill:m==='Speaker'?'white':colors[m],'stroke-width':1.6}))}elem('text',{x:L,y:18,fill:'#536579','font-size':12},'增益 / dB');elem('text',{x:R,y:B+48,'text-anchor':'end',fill:'#536579','font-size':12},'频率 / Hz（对数轴）');let lx=L;for(let m of shown){elem('text',{x:lx,y:416,fill:colors[m],'font-size':13},labels[m]+'软件频响'+($('markers').checked?' / 原厂增益点':''));lx+=300}$('name').textContent=D.labels[current]+' / '+current;let same=JSON.stringify(p.settings.Speaker)===JSON.stringify(p.settings.Headphone);$('status').textContent=D.categories[current]+' · Type '+p.type+' · PreAmp：扬声器 '+p.settings.Speaker.preamp+' dB / 耳机 '+p.settings.Headphone.preamp+' dB · '+(same?'两种输出设置相同':'两种输出设置不同')+' · 当前 '+sr+' Hz / Q=1 / 附加 '+gain+' dB';let table=document.createElement('table');let h=document.createElement('tr');for(let t of ['原厂增益 dB',...p.settings.Speaker.freqs.map(f=>f>=1000?f/1000+'k':f)]){let e=document.createElement('th');e.textContent=t;h.appendChild(e)}table.appendChild(h);for(let m of ['Speaker','Headphone']){let row=document.createElement('tr');for(let t of [labels[m],...p.settings[m].values]){let e=document.createElement('td');e.textContent=t;row.appendChild(e)}table.appendChild(row)}$('bands').replaceChildren(table)}
$('plot').addEventListener('pointermove',e=>{let rect=$('plot').getBoundingClientRect(),px=(e.clientX-rect.left)*1000/rect.width;if(px<L||px>R)return;let f=20*1000**((px-L)/(R-L)),sr=+$('sr').value,g=Math.min(24,Math.max(-24,+$('gain').value||0));$('tip').textContent=f.toFixed(1)+' Hz  '+shown.map(m=>labels[m]+' '+response(D.presets[current].settings[m],f,sr,g).toFixed(3)+' dB').join('  |  ')});
$('preset').onchange=()=>{current=$('preset').value;render()};$('search').oninput=options;for(let id of ['mode','sr','gain','markers'])$(id).oninput=render;$('export').onclick=()=>{let lines=['preset_id,output_mode,sample_rate_hz,Q,extra_gain_db,frequency_hz,response_db'];for(let m of shown)grid.forEach((f,i)=>lines.push([current,m,$('sr').value,1,Math.min(24,Math.max(-24,+$('gain').value||0)),f.toFixed(9),values[m][i].toFixed(9)].join(',')));let url=URL.createObjectURL(new Blob(['\ufeff'+lines.join('\r\n')],{type:'text/csv;charset=utf-8'})),a=document.createElement('a');a.href=url;a.download=current+'-'+$('sr').value+'Hz.csv';a.click();setTimeout(()=>URL.revokeObjectURL(url),1000)};options();render();
</script></html>'''
HTML = HTML.replace("</script></html>", '</script><script src="audio/audio-manifest.js"></script><script src="audio-player.js"></script></html>')
(OUT/"index.html").write_text(HTML.replace("__DATA__",json.dumps(payload,ensure_ascii=False,separators=(",",":"))),encoding="utf-8")
md=["# G6 离线 EQ 模式与频响图册","","日期：2026-11-12（UTC+8）。原始参考文件与产品程序未修改。","","## 文件入口","","- [交互图册](index.html)：搜索预设、扬声器/耳机叠加、切换44.1/48/96k、附加增益、悬停读数、当前曲线CSV。","- [完整PDF](all-modes-catalog.pdf)：34页，每页一个预设，双模式频响与十段原厂值。","- [34模式总览PNG](all-modes-overview.png)；[脚步增强示例PNG](footstep-enhancer.png)。","- [68行原厂增益CSV](eq-modes-and-gains.csv)：34预设×2模式，UTF-8 BOM，便于Excel导入。","- [48k连续频响CSV](eq-frequency-response-48k.csv)：68曲线×1024频点，共69632行数据。","- [结构化数据](eq-catalog-data.json)；[来源哈希与曲线峰值摘要](manifest.json)。","","## 读图口径","","原厂十段频点：31 / 62 / 125 / 250 / 500 / 1000 / 2000 / 4000 / 8000 / 16000 Hz。离散点为JSON原厂增益；平滑线为当前sbc-eq的软件理论频响，默认48 kHz、Q=1、附加增益0 dB并计入预设PreAmp。相邻频段叠加，不能把滑块值连线当成实际频响。","","34组预设共68条输出模式记录，其中18组双模式不同、16组相同。中文名与分类为整理译名，英文ID才是CLI参数。这里是EQ预设：相同名字的SBX profile是另一套效果参数；FootstepEnhancer不等于Scout。","","## 模式索引","","| 中文整理名 | 原厂ID / CLI参数 | 分类 | 扬声器/耳机 |","|---|---|---|---|"]
md[6:6]=["- [本地音频与输入输出设备版本](LOCAL-AUDIO.md)：先双击项目根目录start-eq-lab.cmd，选择自己的音频或麦克风，实时A/B。", "- [预制音频与四组A→B示例](audio/README.md)；[网页验收记录](audio/browser-validation.json)。"]
for n in ORDER:md.append(f"| {LABELS[n]} | {n} | {category(n)} | {'不同' if n in different else '相同'} |")
md.extend(["","## 特别容易混淆的三组","","FPS、CallofDutySeries、DrivingSimulation只有Speaker曲线全零，Headphone均非零。完整原厂增益已分别导出，不能按名字认定为无效果。","","## 复现与验证","","在creative-sbcommand目录运行：`./tools/audio-env/Scripts/python.exe -X utf8 ./tools/build-eq-catalog.py`。计算由SciPy sosfreqz完成，后续对照Rust处理器实际导出的脉冲FFT检查，结果写入validation.json。","","原厂Q、系数拓扑、硬件执行和频响未证实。本图册是当前软件EQ模型的理论频响，不是声卡实测，也不是完整SBX效果。"])
(OUT/"README.md").write_text("\n".join(md)+"\n",encoding="utf-8")
print(json.dumps({"presets":34,"curves":68,"different_pairs":len(different),"same_pairs":34-len(different),"response_rows":68*len(FREQS),"outputs":[p.name for p in OUT.iterdir() if p.is_file()]},ensure_ascii=False,indent=2))
cache.cleanup()
