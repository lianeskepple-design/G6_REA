(() => {
  'use strict';
  const $ = id => document.getElementById(id);
  const S = { phase:'idle', ctx:null, buffer:null, name:'', sourceKind:'file', source:null,
    gainA:null, gainB:null, wetPreamp:null, master:null, filters:[], micStream:null, captureKind:null, displayInfo:null,
    micSource:null, monitoring:false, playing:false, offset:0, started:0,
    generation:0, playGeneration:0, tickToken:0, routeGeneration:0, devices:[],
    sinkNode:null, audioEl:null, sinkMode:'default', outputDeviceId:'', externalActive:false, externalCleanup:null, externalMonitoring:false, externalGeneration:0, externalReady:false, externalInfo:null,
    presetId:'FootstepEnhancer', mode:'Speaker', selection:'A', ready:false };
  let D;
  const set = (phase,message) => { S.phase=phase; $('status').textContent=message||phase; };
  const fmt = value => { value=Math.max(0,Number(value)||0); return `${Math.floor(value/60)}:${String(Math.floor(value%60)).padStart(2,'0')}`; };
  const fail = error => set('error',error.message||String(error));

  async function loadData() {
    if(location.protocol==='file:')throw Error('请先双击 start-eq-lab.cmd，通过 localhost 打开此页；直接双击 HTML 无法可靠加载数据或选择设备。');
    const get=async path=>{const r=await fetch(path);if(!r.ok)throw Error(`${path}: HTTP ${r.status}`);return r.json();};
    const [catalog,manifest]=await Promise.all([get('eq-catalog-data.json'),get('audio/audio-manifest.json')]);
    D=catalog;S.manifest=manifest;
    for(const id of D.order){const option=document.createElement('option');option.value=id;option.textContent=`${D.labels[id]} · ${id}`;$('preset').append(option);}
    S.ready=true;applyPreset();set('idle','待机；尚未创建 AudioContext，也未申请设备权限。');refreshProcesses().catch(error=>{$('captureStatus').textContent=`进程列表不可用：${error.message}`;});
  }

  function ensureContext() {
    if(!S.ready)throw Error('EQ 数据尚未就绪，请通过本地启动器打开页面并等待加载完成。');
    if(S.ctx&&S.ctx.state!=='closed')return S.ctx;
    const Constructor=window.AudioContext||window.webkitAudioContext;
    if(!Constructor)throw Error('此浏览器不支持 Web Audio。');
    const ctx=new Constructor();S.ctx=ctx;S.gainA=ctx.createGain();S.gainB=ctx.createGain();
    S.gainA.gain.value=0;S.gainB.gain.value=0;S.filters=[];
    let previous=S.gainB;
    for(let i=0;i<10;i++){const filter=ctx.createBiquadFilter();filter.type='peaking';filter.Q.value=1;previous.connect(filter);previous=filter;S.filters.push(filter);}
    S.wetPreamp=ctx.createGain();S.master=ctx.createGain();
    previous.connect(S.wetPreamp);S.wetPreamp.connect(S.master);S.gainA.connect(S.master);S.master.connect(ctx.destination);
    S.sinkMode='default';S.outputDeviceId='';S.sinkNode=null;S.audioEl=null;
    applyPreset();return ctx;
  }

  function applyPreset(id=S.presetId,mode=$('mode').value) {
    if(!S.ready)return;
    const setting=D.presets[id]?.settings[mode];
    if(!setting||setting.freqs.length!==10||setting.values.length!==10)throw Error('未知或损坏的预设/输出模式。');
    const extra=Number($('extra').value),volume=Number($('volume').value),nyquist=S.ctx?S.ctx.sampleRate/2:Infinity;
    if(!Number.isFinite(extra)||extra< -24||extra>24)throw Error('额外增益必须是 -24 至 +24 dB 的有限值。');
    if(!Number.isFinite(volume)||volume<0||volume>1||!Number.isFinite(setting.preamp))throw Error('音量或前级增益无效。');
    for(let i=0;i<10;i++){
      const frequency=setting.freqs[i],gain=setting.values[i];
      if(!Number.isFinite(frequency)||frequency<=0||!Number.isFinite(gain))throw Error('频点或频段增益无效。');
      if(frequency>=nyquist&&gain!==0)throw Error(`${id} 的 ${frequency} Hz 非零频段超出当前 Nyquist ${nyquist} Hz；未应用。`);
    }
    S.presetId=id;S.mode=mode;$('preset').value=id;$('mode').value=mode;
    if(S.ctx){
      for(let i=0;i<10;i++){const node=S.filters[i];node.frequency.value=Math.min(setting.freqs[i],nyquist*.999999);node.gain.value=setting.values[i];node.Q.value=1;}
      S.wetPreamp.gain.value=10**(setting.preamp/20);
      S.master.gain.setTargetAtTime(volume*10**(extra/20),S.ctx.currentTime,.01);
    }
    $('presetInfo').textContent=`${D.labels[id]} / ${mode==='Speaker'?'扬声器':'耳机'} · Preamp ${setting.preamp} dB · Q=1 · ${S.ctx?'实际 AudioContext '+S.ctx.sampleRate+' Hz':'加载音频后显示实际采样率'}。曲线含前级与额外增益，不含共享主音量。`;
    draw();
  }

  function draw() {
    const canvas=$('response'),pen=canvas.getContext('2d'),w=canvas.width,h=canvas.height;
    pen.clearRect(0,0,w,h);if(!S.ctx||!S.filters.length)return;
    const n=300,maxHz=Math.min(20000,S.ctx.sampleRate/2*.9999),frequencies=new Float32Array(n),magnitude=new Float32Array(n),phase=new Float32Array(n),total=new Float64Array(n).fill(0);
    const extra=Number($('extra').value)||0,preamp=D.presets[S.presetId].settings[S.mode].preamp;
    for(let i=0;i<n;i++)frequencies[i]=20*(maxHz/20)**(i/(n-1));
    for(const filter of S.filters){filter.getFrequencyResponse(frequencies,magnitude,phase);for(let i=0;i<n;i++)total[i]+=20*Math.log10(Math.max(magnitude[i],1e-20));}
    for(let i=0;i<n;i++)total[i]+=preamp+extra;
    const lo=Math.min(-18,Math.floor(Math.min(...total,extra)/6)*6),hi=Math.max(18,Math.ceil(Math.max(...total,extra)/6)*6);
    const px=f=>42+(w-58)*Math.log(f/20)/Math.log(maxHz/20),py=db=>h-28-(db-lo)/(hi-lo)*(h-48);
    pen.font='11px system-ui';pen.fillStyle='#526477';pen.strokeStyle='#d5e0e2';
    for(let db=lo;db<=hi;db+=6){const y=py(db);pen.beginPath();pen.moveTo(42,y);pen.lineTo(w-16,y);pen.stroke();pen.fillText(`${db}`,5,y+4);}
    for(const frequency of [31,125,500,2000,8000,16000])if(frequency<maxHz){const x=px(frequency);pen.fillText(frequency>=1000?frequency/1000+'k':String(frequency),x-10,h-8);}
    pen.strokeStyle='#df7c20';pen.beginPath();pen.moveTo(42,py(extra));pen.lineTo(w-16,py(extra));pen.stroke();
    pen.strokeStyle='#078797';pen.lineWidth=2;pen.beginPath();for(let i=0;i<n;i++){const x=px(frequencies[i]),y=py(total[i]);i?pen.lineTo(x,y):pen.moveTo(x,y);}pen.stroke();
    pen.fillText('dB · 青色 EQ B / 橙色原音 A',42,12);
  }

  function gates(immediate=false) {
    if(!S.ctx)return;
    const allowed=(!S.micStream&&!S.externalActive)||S.monitoring||S.externalMonitoring,now=S.ctx.currentTime;
    for(const [node,selected] of [[S.gainA,S.selection==='A'],[S.gainB,S.selection==='B']]){
      const value=allowed&&selected?1:0;node.gain.cancelScheduledValues(now);
      if(immediate)node.gain.setValueAtTime(value,now);else node.gain.setTargetAtTime(value,now,.015);
    }
  }
  function muteGates(){if(S.ctx)for(const node of [S.gainA,S.gainB]){node.gain.cancelScheduledValues(S.ctx.currentTime);node.gain.setValueAtTime(0,S.ctx.currentTime);}}
  function stopSource(reset=true) {
    const old=S.source;S.source=null;S.playing=false;S.tickToken++;
    if(old){old.onended=null;try{old.stop();}catch{}try{old.disconnect();}catch{}}
    if(reset)S.offset=0;
  }
  function releaseMic() {
    muteGates();const stream=S.micStream;S.micStream=null;
    if(S.micSource){S.micSource.disconnect();S.micSource=null;}
    if(stream)stream.getTracks().forEach(track=>track.stop());
    S.monitoring=false;$('monitor').disabled=true;$('stopMic').disabled=true;$('monitor').textContent='开始监听';
  }
  function updateTime() {const d=S.buffer?.duration||0;let position=S.offset;if(S.playing)position+=S.ctx.currentTime-S.started;if(d)position=$('loop').checked?position%d:Math.min(position,d);$('seek').value=d?String(Math.round(position/d*1000)):'0';$('time').textContent=`${fmt(position)} / ${fmt(d)}`;}
  function tick(token){if(token!==S.tickToken||!S.playing)return;updateTime();requestAnimationFrame(()=>tick(token));}
  async function outputReady(){await S.ctx.resume();if(S.sinkMode==='element')await S.audioEl.play();}
  function startSource() {
    if(!S.buffer||S.buffer.duration<=0)throw Error('请先加载有效音频。');
    stopSource(false);const source=S.ctx.createBufferSource();source.buffer=S.buffer;source.loop=$('loop').checked;
    source.connect(S.gainA);source.connect(S.gainB);S.source=source;
    if(S.offset>=S.buffer.duration)S.offset=0;
    gates(true);S.started=S.ctx.currentTime;source.start(0,Math.max(0,S.offset));S.playing=true;
    source.onended=()=>{if(S.source===source&&!source.loop){stopSource(true);updateTime();set('ready','播放结束。');}};
    set('playing',`播放中 · ${S.name} · ${S.ctx.sampleRate} Hz`);const token=++S.tickToken;tick(token);
  }
  async function playFile() {
    if(!S.buffer)throw Error('请先选择本地文件或加载诊断音。');
    const token=++S.playGeneration;releaseMic();stopExternal();S.sourceKind='file';ensureContext();applyPreset();
    await outputReady();if(token!==S.playGeneration)return;
    $('pause').disabled=false;$('seek').disabled=false;
    $('fileInfo').textContent=`${S.name} · ${S.buffer.duration.toFixed(2)} 秒 · ${S.buffer.numberOfChannels} 声道 · AudioContext ${S.ctx.sampleRate} Hz`;
    if(!S.playing)startSource();else gates();
  }
  function pauseFile(){++S.playGeneration;if(S.playing){S.offset+=S.ctx.currentTime-S.started;if($('loop').checked&&S.buffer)S.offset%=S.buffer.duration;stopSource(false);}updateTime();set('paused','已暂停，保留当前位置。');}

  async function decode(buffer,name,kind,token=++S.generation) {
    ++S.playGeneration;stopSource();releaseMic();stopExternal();S.sourceKind=kind;set('loading',`解码 ${name}…`);
    $('play').disabled=true;$('pause').disabled=true;$('seek').disabled=true;
    try{
      const ctx=ensureContext(),decoded=await ctx.decodeAudioData(buffer.slice(0));
      if(token!==S.generation)return false;
      if(!decoded.length)throw Error('音频没有可播放的采样。');
      S.buffer=decoded;S.name=name;S.offset=0;
      $('fileInfo').textContent=`${name} · ${decoded.duration.toFixed(2)} 秒 · ${decoded.numberOfChannels} 声道 · 播放/处理采样率 ${ctx.sampleRate} Hz（浏览器可能重采样）`;
      $('play').disabled=false;$('pause').disabled=false;$('seek').disabled=false;applyPreset();updateTime();
      set('ready','音源已就绪；点击播放，不会自动播放。');return true;
    }catch(error){if(token===S.generation){$('play').disabled=!S.buffer;$('pause').disabled=!S.buffer;set('error',`解码失败：${error.message}`);}throw error;}
  }
  const loadFileBuffer=(buffer,name='本地音频')=>decode(buffer,name,'file');
  async function fixture(){const token=++S.generation;++S.playGeneration;stopSource();releaseMic();stopExternal();const path=S.manifest?.original?.path;if(!path)throw Error('清单中缺少原音路径。');set('loading','读取合成诊断音…');const response=await fetch(path);if(!response.ok)throw Error(`诊断音读取失败 HTTP ${response.status}`);const bytes=await response.arrayBuffer();if(token!==S.generation)return;return decode(bytes,'合成诊断音','fixture',token);}

  async function refreshDevices() {
    if(!navigator.mediaDevices?.enumerateDevices){$('deviceStatus').textContent='设备枚举不可用；请通过本地启动器使用 localhost/HTTPS 页面。';return [];}
    S.devices=await navigator.mediaDevices.enumerateDevices();
    for(const [id,kind] of [['input','audioinput'],['output','audiooutput']]){
      const select=$(id),old=select.value;select.replaceChildren();
      const defaultOption=document.createElement('option');defaultOption.value='';defaultOption.textContent=kind==='audioinput'?'系统默认输入':'系统默认输出';select.append(defaultOption);
      let count=0;for(const device of S.devices.filter(item=>item.kind===kind)){const option=document.createElement('option');option.value=device.deviceId;option.textContent=device.label||`未命名${kind==='audioinput'?'输入':'输出'}设备 ${++count}（名称需授权）`;select.append(option);}
      if([...select.options].some(option=>option.value===old))select.value=old;
    }
    return S.devices;
  }
  async function authorize(){if(!navigator.mediaDevices?.getUserMedia)throw Error('麦克风权限接口不可用；请使用 localhost/HTTPS。');let stream;try{stream=await navigator.mediaDevices.getUserMedia({audio:true});}finally{stream?.getTracks().forEach(track=>track.stop());}await refreshDevices();$('deviceStatus').textContent='已刷新授权后的设备名称；临时麦克风轨道已停止。';}
  async function routeOutput(id) {
    const token=++S.routeGeneration,ctx=ensureContext();const previous=S.outputDeviceId;
    try{
      if(typeof ctx.setSinkId==='function'){
        await ctx.setSinkId(id||'');if(token!==S.routeGeneration)return;
        if(S.sinkMode==='element'){S.audioEl.pause();S.master.disconnect();S.master.connect(ctx.destination);}
        S.sinkMode='context';
      }else if(typeof HTMLMediaElement.prototype.setSinkId==='function'){
        if(!S.sinkNode){S.sinkNode=ctx.createMediaStreamDestination();S.audioEl=new Audio();S.audioEl.srcObject=S.sinkNode.stream;S.audioEl.autoplay=false;}
        await S.audioEl.setSinkId(id||'');if(token!==S.routeGeneration)return;
        if(S.sinkMode!=='element'){S.master.disconnect();S.master.connect(S.sinkNode);}
        S.sinkMode='element';if(S.playing||S.monitoring)await S.audioEl.play();
      }else{
        $('output').value='';S.outputDeviceId='';S.sinkMode='default';$('deviceStatus').textContent='此浏览器不支持指定输出设备，使用系统默认输出。';return false;
      }
      S.outputDeviceId=id||'';$('deviceStatus').textContent=`已通过 ${S.sinkMode==='context'?'AudioContext.setSinkId':'HTMLAudioElement.setSinkId'} 路由至 ${$('output').selectedOptions[0]?.textContent||'选定输出'}。`;return true;
    }catch(error){if(token===S.routeGeneration){$('output').value=previous;$('deviceStatus').textContent=`输出路由失败，保留原路由：${error.message}`;}throw error;}
  }
  async function chooseOutput(){if(!navigator.mediaDevices?.selectAudioOutput){$('deviceStatus').textContent='此浏览器没有系统输出选择器；可刷新设备后使用输出列表。指定输出还需 setSinkId 支持。';return;}const device=await navigator.mediaDevices.selectAudioOutput();await refreshDevices();if(![...$('output').options].some(option=>option.value===device.deviceId)){const option=document.createElement('option');option.value=device.deviceId;option.textContent=device.label||'已授权输出';$('output').append(option);}$('output').value=device.deviceId;await routeOutput(device.deviceId);}

  async function enableMic() {
    if(!S.ready)throw Error('EQ 数据尚未就绪，未申请麦克风权限。');
    const token=++S.generation;++S.playGeneration;stopSource();releaseMic();stopExternal();
    if(!navigator.mediaDevices?.getUserMedia)throw Error('麦克风不可用，请使用 localhost/HTTPS 并允许权限。');
    set('permission','等待麦克风权限；尚未监听。');$('stopMic').disabled=false;
    const deviceId=$('input').value;let stream;
    try{
      stream=await navigator.mediaDevices.getUserMedia({audio:{...(deviceId?{deviceId:{exact:deviceId}}:{}),echoCancellation:false,noiseSuppression:false,autoGainControl:false}});
      if(token!==S.generation){stream.getTracks().forEach(track=>track.stop());return;}
      ensureContext();applyPreset();muteGates();S.micStream=stream;S.monitoring=false;S.sourceKind='mic';
      S.micSource=S.ctx.createMediaStreamSource(stream);S.micSource.connect(S.gainA);S.micSource.connect(S.gainB);
      for(const track of stream.getTracks())track.addEventListener('ended',()=>{if(S.micStream===stream){stopMic();set('error','麦克风设备已断开，轨道已释放。');}});
      $('monitor').disabled=false;$('stopMic').disabled=false;$('pause').disabled=true;$('seek').disabled=true;
      $('fileInfo').textContent=`当前输入：麦克风 · AudioContext ${S.ctx.sampleRate} Hz；文件播放已停止。`;
      set('captured','麦克风已捕获，监听关闭；点击开始监听才会输出。');
    }catch(error){stream?.getTracks().forEach(track=>track.stop());if(token===S.generation){releaseMic();set('error',`麦克风启动失败：${error.message}`);}throw error;}
  }
  function stopExternal(message='捕获已停止，连接、辅助进程和音轨已释放。') {
    ++S.externalGeneration;const cleanup=S.externalCleanup;S.externalCleanup=null;
    S.externalActive=false;S.externalReady=false;S.externalMonitoring=false;S.externalInfo=null;
    muteGates();try{cleanup?.();}catch{}
    $('monitorCapture').disabled=true;$('stopCapture').disabled=true;$('monitorCapture').textContent='开始监听';
    $('captureStatus').textContent=message;
    if(S.sourceKind==='desktop-process'||S.sourceKind==='display-audio')S.sourceKind='file';
  }
  function prepareExternal(kind) {
    if(!S.ready)throw Error('EQ数据尚未就绪，请通过本地启动器打开。');
    ++S.generation;++S.playGeneration;stopSource();releaseMic();stopExternal();
    const token=++S.externalGeneration;S.sourceKind=kind;ensureContext();applyPreset();muteGates();
    S.externalActive=true;S.externalReady=false;S.externalMonitoring=false;S.selection='B';$('source').value='B';
    S.externalContextReady=S.ctx.resume();S.externalContextReady.catch(()=>{});
    $('stopCapture').disabled=false;$('monitorCapture').disabled=true;$('pause').disabled=true;$('seek').disabled=true;
    set('connecting','准备外部音频；尚未监听。');
    return ()=>S.externalActive&&S.externalGeneration===token;
  }
  function finishExternal(current,info,message) {
    if(!current())return false;
    S.externalInfo=info;S.externalReady=true;$('monitorCapture').disabled=false;
    $('captureStatus').textContent=message;$('fileInfo').textContent=`当前输入：${info.label}；文件播放和麦克风已停止。`;
    set('captured','外部音频已就绪，监听关闭；点击“开始监听”才会输出。');return true;
  }
  async function connectProcess() {
    const pid=Number($('processList').value);
    if(!Number.isInteger(pid)||pid<=0||pid>0xffffffff)throw Error('请先刷新并选择客户端进程。');
    const current=prepareExternal('desktop-process');let ws=null,node=null,timer=null,rejectReady=null,closed=false;
    const cleanup=()=>{
      if(closed)return;closed=true;clearTimeout(timer);rejectReady?.(Error('捕获已取消。'));
      if(ws){ws.onmessage=null;ws.onerror=null;ws.onclose=null;try{if(ws.readyState===WebSocket.OPEN)ws.send('stop');ws.close();}catch{}}
      if(node){try{node.port.postMessage({type:'stop'});node.port.close();node.disconnect();}catch{}}
    };
    S.externalCleanup=cleanup;
    try{
      const response=await fetch('/api/session');if(!response.ok)throw Error(`原生服务 HTTP ${response.status}`);
      const session=await response.json();if(!current())return false;
      if(!session.capabilities?.available||session.capabilities.supported===false)throw Error(session.capabilities?.reason||'原生helper尚未构建或系统不支持。');
      await S.externalContextReady;if(!current())return false;
      if(!S.ctx.audioWorklet)throw Error('此浏览器没有AudioWorklet，请使用Chrome/Edge的localhost页面。');
      if(S.workletContext!==S.ctx){await S.ctx.audioWorklet.addModule('process-capture-worklet.js');if(!current())return false;S.workletContext=S.ctx;}
      node=new AudioWorkletNode(S.ctx,'process-capture',{numberOfInputs:0,numberOfOutputs:1,outputChannelCount:[2]});node.connect(S.gainA);node.connect(S.gainB);
      const ready=new Promise((resolve,reject)=>{
        rejectReady=reject;
        timer=setTimeout(()=>reject(Error('原生采集初始化超时。')),17000);
        ws=new WebSocket(`ws://${location.host}/api/process-audio?pid=${encodeURIComponent(pid)}&token=${encodeURIComponent(session.token)}`);ws.binaryType='arraybuffer';
        ws.onmessage=event=>{
          if(!current())return;
          if(typeof event.data==='string'){
            let item;try{item=JSON.parse(event.data);}catch{return;}
            if(item.event==='ready'){
              if(item.channels!==2||item.sample_format!=='f32le'||!Number.isFinite(item.sample_rate)||item.sample_rate<8000||item.sample_rate>384000){reject(Error('原生PCM格式不受支持。'));return;}
              clearTimeout(timer);node.port.postMessage({type:'ready',sample_rate:item.sample_rate});
              const label=$('processList').selectedOptions[0]?.textContent||`PID ${pid}`;
              resolve({label,sample_rate:item.sample_rate,pid,kind:'process'});
            }else if(item.event==='error'||item.event==='ended'){
              const message=`进程采集结束：${item.message||'helper已退出'}${item.hresult?' / '+item.hresult:''}`;
              reject(Error(message));stopExternal(message);set('error',message);
            }
          }else{
            if(event.data.byteLength%8){reject(Error('原生PCM帧不完整。'));return;}
            if(S.externalInfo?.kind==='process')S.externalInfo.frames_received=(S.externalInfo.frames_received||0)+event.data.byteLength/8;
            node.port.postMessage({type:'pcm',buffer:event.data},[event.data]);
          }
        };
        ws.onerror=()=>reject(Error('无法连接进程音频流；请确认本地服务和原生helper可用。'));
        ws.onclose=event=>{if(current()){const message=`进程音频连接已断开（${event.code}）。`;reject(Error(message));stopExternal(message);set('error',message);}};
      });
      ready.catch(()=>{});
      node.port.onmessage=event=>{
        if(!current()||!S.externalReady||event.data.type!=='buffer')return;
        const stats=event.data,rate=S.externalInfo.sample_rate;
        const signal=!stats.received_frames?'等待目标播放声音':stats.packet_age_frames>S.ctx.sampleRate?'暂无新音频（目标可能暂停）':stats.input_peak>1e-7?'收到音频信号':'收到静音/极低电平';
        $('captureStatus').textContent=`${signal} · ${S.externalInfo.label} · 输入 ${rate} Hz → EQ ${S.ctx.sampleRate} Hz · 缓冲 ${(1000*(stats.queued_frames||0)/rate).toFixed(0)} ms · 欠载采样 ${stats.underruns||0} · 丢旧采样 ${stats.dropped||0}（采样率不同时采用线性插值）`;
      };
      const info=await ready;
      return finishExternal(current,info,`${info.label} · 进程树采集已就绪 · ${info.sample_rate} Hz；目标暂停时保持连接，监听默认关闭。`);
    }catch(error){if(current()){stopExternal(`连接失败：${error.message}`);set('error',error.message);throw error;}return false;}
    finally{if(!current())cleanup();}
  }
  async function connectShared() {
    if(!navigator.mediaDevices?.getDisplayMedia)throw Error('此浏览器不支持网页/屏幕音频共享；请使用Chrome或Edge的localhost页面。');
    const current=prepareExternal('display-audio');let stream=null,source=null;
    S.externalCleanup=()=>{try{source?.disconnect();}catch{}stream?.getTracks().forEach(track=>track.stop());};
    try{
      $('captureStatus').textContent='请选择需要的浏览器标签页（例如B站）并勾选共享标签页音频。';
      stream=await navigator.mediaDevices.getDisplayMedia({video:{displaySurface:'browser'},audio:{echoCancellation:false,noiseSuppression:false,autoGainControl:false,suppressLocalAudioPlayback:true},selfBrowserSurface:'exclude',systemAudio:'include',surfaceSwitching:'include'});
      if(!current()){stream.getTracks().forEach(track=>track.stop());return false;}
      await S.externalContextReady;if(!current()){stream.getTracks().forEach(track=>track.stop());return false;}
      const audio=stream.getAudioTracks()[0];
      if(!audio)throw Error('没有返回音频轨道；本次共享已停止。请选择标签页并勾选共享音频，单独共享窗口可能不带音频。');
      source=S.ctx.createMediaStreamSource(new MediaStream([audio]));source.connect(S.gainA);source.connect(S.gainB);
      const settings=audio.getSettings(),surface=stream.getVideoTracks()[0]?.getSettings().displaySurface;
      const suppression=settings.suppressLocalAudioPlayback===true?'浏览器确认已抑制原声':settings.suppressLocalAudioPlayback===false?'未抑制原声，可能叠音':'原声抑制未知，可能叠音';
      const info={label:surface==='browser'?'共享标签页声音':'共享屏幕/系统混音',kind:'display',sample_rate:settings.sampleRate||S.ctx.sampleRate,displaySurface:surface||'unknown',localPlaybackSuppressed:settings.suppressLocalAudioPlayback===true};
      S.displayInfo=info;
      for(const track of stream.getTracks())track.addEventListener('ended',()=>{if(current()){stopExternal('浏览器共享已结束，全部音轨已释放。');set('ready','共享已停止。');}});
      return finishExternal(current,info,`${info.label} · ${info.sample_rate} Hz · ${suppression}。画面不绘制/上传；保留视频轨道仅用于维持共享，会在停止时一起释放。`);
    }catch(error){stream?.getTracks().forEach(track=>track.stop());if(current()){stopExternal(`共享失败：${error.message}`);set('error',error.message);throw error;}return false;}
  }
  function renderProcessList() {
    const select=$('processList'),old=select.value,query=$('processFilter').value.trim().toLowerCase();select.replaceChildren();
    const placeholder=document.createElement('option');placeholder.value='';placeholder.textContent='请选择客户端进程（名称 / PID）';select.append(placeholder);
    for(const process of S.processCandidates||[]){
      if(query&&!(`${process.name} ${process.pid}`.toLowerCase().includes(query)))continue;
      const option=document.createElement('option');option.value=String(process.pid);option.textContent=`${process.name} · PID ${process.pid}${process.has_window?'':' · 后台'}`;select.append(option);
    }
    if([...select.options].some(option=>option.value===old))select.value=old;
    if(select.options.length===1)placeholder.textContent=query?'没有匹配的客户端，请修改搜索条件':'未找到客户端，请启动目标程序后刷新';
  }
  async function refreshProcesses() {
    const response=await fetch('/api/processes');if(!response.ok)throw Error(`进程列表 HTTP ${response.status}`);
    const data=await response.json();S.processCandidates=data.available?data.processes:[];renderProcessList();
    if(!data.available&&!S.externalActive)$('captureStatus').textContent=data.reason||'原生helper未构建/不可用。';
    return data;
  }
  async function monitorExternal() {
    if(!S.externalActive||!S.externalReady)return;
    if(S.externalMonitoring){S.externalMonitoring=false;muteGates();$('monitorCapture').textContent='开始监听';set('captured','仍在捕获，监听已关闭。');return;}
    const token=S.externalGeneration,playToken=++S.playGeneration;await outputReady();
    if(!S.externalActive||!S.externalReady||token!==S.externalGeneration||playToken!==S.playGeneration)return;
    S.externalMonitoring=true;gates();$('monitorCapture').textContent='停止监听';set('monitoring','正在监听所选外部音频的EQ B/原音 A。');
  }
  function stopMic(){++S.generation;++S.playGeneration;releaseMic();S.sourceKind='file';set(S.buffer?'ready':'idle','麦克风已停止，所有轨道已释放。');}
  async function monitor(){if(!S.micStream)throw Error('请先启用麦克风。');const token=++S.playGeneration;if(S.monitoring){S.monitoring=false;muteGates();$('monitor').textContent='开始监听';set('captured','麦克风捕获中，监听关闭。');return;}await outputReady();if(token!==S.playGeneration||!S.micStream)return;S.monitoring=true;gates();$('monitor').textContent='停止监听';set('monitoring','正在监听麦克风；请使用耳机。');}
  function setSourceMode(value){if(value!=='A'&&value!=='B')throw Error('音源必须为 A 或 B。');applyPreset();S.selection=value;$('source').value=value;gates();}
  function stop(){++S.generation;++S.playGeneration;stopSource();releaseMic();stopExternal();S.sourceKind='file';S.audioEl?.pause();S.ctx?.suspend().catch(()=>{});updateTime();set(S.buffer?'ready':'idle','已停止。');}
  async function dispose(){stop();if(S.audioEl){S.audioEl.pause();S.audioEl.srcObject=null;}if(S.sinkNode)S.sinkNode.stream.getTracks().forEach(track=>track.stop());if(S.ctx){await S.ctx.close();S.ctx=null;}}

  $('file').onchange=async event=>{const file=event.target.files?.[0];if(!file)return;const token=++S.generation;++S.playGeneration;stopSource();releaseMic();stopExternal();try{const bytes=await file.arrayBuffer();if(token===S.generation)await decode(bytes,file.name,'file',token);}catch(error){if(token===S.generation)fail(error);}};
  $('fixture').onclick=()=>fixture().catch(fail);$('play').onclick=()=>playFile().catch(fail);$('pause').onclick=pauseFile;
  $('loop').onchange=()=>{if(S.source)S.source.loop=$('loop').checked;};
  $('seek').oninput=()=>{if(!S.buffer)return;const wasPlaying=S.playing;stopSource(false);S.offset=Number($('seek').value)/1000*S.buffer.duration;if(wasPlaying)startSource();else updateTime();};
  $('source').onchange=event=>{try{setSourceMode(event.target.value);}catch(error){fail(error);}};
  $('preset').onchange=()=>{try{applyPreset($('preset').value,$('mode').value);}catch(error){$('preset').value=S.presetId;fail(error);}};
  $('mode').onchange=()=>{try{applyPreset(S.presetId,$('mode').value);}catch(error){$('mode').value=S.mode;fail(error);}};
  for(const id of ['extra','volume'])$(id).addEventListener(id==='volume'?'input':'change',()=>{try{applyPreset();}catch(error){fail(error);}});
  $('refresh').onclick=()=>refreshDevices().then(()=>{$('deviceStatus').textContent='设备列表已刷新；没有申请麦克风权限。';}).catch(error=>{$('deviceStatus').textContent=`枚举失败：${error.message}`;});
  $('authorize').onclick=()=>authorize().catch(error=>{$('deviceStatus').textContent=`授权失败：${error.message}`;});
  $('chooseOutput').onclick=()=>chooseOutput().catch(error=>{$('deviceStatus').textContent=`输出选择失败：${error.message}`;});
  $('output').onchange=()=>routeOutput($('output').value).catch(()=>{});
  $('mic').onclick=()=>enableMic().catch(()=>{});$('monitor').onclick=()=>monitor().catch(fail);$('stopMic').onclick=stopMic;
  $('processFilter').oninput=renderProcessList;
  $('refreshProcesses').onclick=()=>refreshProcesses().catch(error=>{$('captureStatus').textContent=`进程列表失败：${error.message}`;});$('connectProcess').onclick=()=>connectProcess().catch(error=>{$('captureStatus').textContent=`连接失败：${error.message}`;fail(error);});$('shareTab').onclick=()=>connectShared().catch(error=>{$('captureStatus').textContent=`共享失败：${error.message}`;fail(error);});$('monitorCapture').onclick=()=>monitorExternal().catch(fail);$('stopCapture').onclick=()=>stopExternal();
  $('input').onchange=()=>{if(S.micStream)enableMic().catch(()=>{});};
  navigator.mediaDevices?.addEventListener?.('devicechange',()=>{const selected=S.outputDeviceId;refreshDevices().then(()=>{if(selected&&!S.devices.some(device=>device.kind==='audiooutput'&&device.deviceId===selected))return routeOutput('').then(()=>{$('deviceStatus').textContent='原输出设备已移除，已切回系统默认输出。';});}).catch(error=>{$('deviceStatus').textContent=`设备变更处理失败：${error.message}`;});});
  window.addEventListener('pagehide',()=>{dispose().catch(()=>{});});
  window.eqLab={state:S,loadFileBuffer,selectPreset:id=>applyPreset(id,$('mode').value),setSourceMode,refreshDevices,routeOutput,enableMic,stopMic,stop,dispose,connectProcess,connectShared,stopExternal,monitorExternal,refreshProcesses};
  loadData().catch(fail);
})();
