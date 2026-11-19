(() => {
  'use strict';
  const manifest = window.EQ_AUDIO_DATA;
  const host = document.createElement('section');
  host.id = 'eq-audio-player';
  host.setAttribute('aria-labelledby', 'eq-audio-title');
  host.innerHTML = `<style>
    #eq-audio-player{margin:20px 0 8px;padding:16px;border:1px solid #dbe3ea;border-radius:12px;background:#f8fbfc}
    #eq-audio-player h3{margin:0 0 6px;font-size:19px;color:#172738}#eq-audio-player p{margin:6px 0;color:#526477;font-size:13px}
    #eq-audio-player .eq-audio-row{display:flex;flex-wrap:wrap;gap:10px;align-items:center;margin-top:10px}
    #eq-audio-player label{font-size:13px}#eq-audio-player button{cursor:pointer}#eq-audio-player button[aria-pressed="true"]{background:#e5f2f4;border-color:#147d92}
    #eq-audio-player .eq-audio-status{min-height:1.6em;color:#455c6d;font-size:13px}#eq-audio-player audio{display:none}
    #eq-audio-player .eq-audio-vol{width:min(180px,42vw);vertical-align:middle}#eq-audio-player .eq-audio-badge{font-size:12px;color:#147d92}
  </style>
  <h3 id="eq-audio-title">原音 A / EQ B 对比试听</h3>
  <p>本地合成诊断素材，非真实游戏录音。试听音频为预渲染文件（48 kHz、Q=1、附加增益 <span data-extra-gain></span> dB）；上方曲线的采样率与附加增益只改变曲线显示，不改变试听音频。</p>
  <div class="eq-audio-row"><span class="eq-audio-badge" data-choice></span><label data-mode-choice hidden>输出模式 <select data-mode><option value="Headphone">耳机</option><option value="Speaker">扬声器</option></select></label><button type="button" data-ab="original" aria-pressed="true">A 原音</button><button type="button" data-ab="eq" aria-pressed="false">B EQ</button><button type="button" data-toggle>播放</button><label>音量 <input class="eq-audio-vol" data-volume type="range" min="0" max="1" step="0.01" value="0.8"></label><label><input data-loop type="checkbox">循环</label><button type="button" data-params>曲线使用试听参数</button><a data-download download>下载当前音频</a></div>
  <div class="eq-audio-row"><input data-seek type="range" min="0" max="1000" step="1" value="0" style="flex:1;min-width:150px" aria-label="播放进度"><span data-time>00:00 / 00:00</span></div>
  <div class="eq-audio-status" role="status" aria-live="polite" data-status></div><audio data-audio="original" preload="none"></audio><audio data-audio="eq" preload="none"></audio>`;
  const main = document.querySelector('main');
  const plot = document.getElementById('plot');
  main.insertBefore(host, plot);
  const $ = (s) => host.querySelector(s);
  const audios = { original: $('[data-audio="original"]'), eq: $('[data-audio="eq"]') };
  const status = $('[data-status]');
  let active = 'original', seekDrag = false, knownPreset = '', knownMode = '';
  const valid = manifest && typeof manifest === 'object' && manifest.presets && typeof manifest.presets === 'object';
  const cleanPath = (entry) => typeof entry === 'string' ? entry : entry && typeof entry.path === 'string' ? entry.path : '';
  const setStatus = (text) => { status.textContent = text; };
  const durationLabel = (n) => { if (!Number.isFinite(n) || n < 0) n = 0; return `${String(Math.floor(n / 60)).padStart(2,'0')}:${String(Math.floor(n % 60)).padStart(2,'0')}`; };
  function selectedMode() { return document.getElementById('mode').value === 'both' ? $('[data-mode]').value : document.getElementById('mode').value; }
  function paths() {
    if (!valid) return {};
    const preset = manifest.presets[current];
    const mode = selectedMode();
    return { original: cleanPath(manifest.original), eq: cleanPath(preset && preset[mode]), mode };
  }
  function syncDownload() {
    const src = audios[active].getAttribute('src');
    const link = $('[data-download]');
    if (src) { link.href = src; link.download = src.split('/').pop() || 'audio.wav'; link.textContent = `下载当前${active === 'original' ? '原音 A' : '效果 B'}`; link.setAttribute('aria-disabled','false'); }
    else { link.removeAttribute('href'); link.removeAttribute('download'); link.textContent = '下载当前音频（素材缺失）'; link.setAttribute('aria-disabled','true'); }
  }
  function setActive(next) {
    if (next === active) return;
    const from = audios[active], to = audios[next], pos = Number.isFinite(from.currentTime) ? from.currentTime : 0, wasPlaying = !from.paused;
    from.pause(); active = next;
    const source = to.getAttribute('src');
    if (!source) { setStatus(`缺少${next === 'original' ? '原音' : '当前预设效果'}素材。`); }
    else {
      if (to.readyState === 0) to.load();
      const apply = () => { if (active !== next || to.getAttribute('src') !== source) return; try { to.currentTime = Math.min(pos, Number.isFinite(to.duration) ? to.duration : pos); } catch (_) {} if (wasPlaying) playActive(); };
      if (to.readyState >= 1) apply(); else to.addEventListener('loadedmetadata', apply, { once:true });
    }
    updateButtons(); syncDownload(); updateTime();
  }
  function updateButtons() { host.querySelectorAll('[data-ab]').forEach(b => b.setAttribute('aria-pressed', String(b.dataset.ab === active))); $('[data-toggle]').textContent = audios[active].paused ? '播放' : '暂停'; }
  async function playActive() {
    const a = audios[active];
    Object.values(audios).forEach(other => { if (other !== a) other.pause(); });
    if (!a.getAttribute('src')) { setStatus(`当前${active === 'original' ? '原音' : '效果'}素材缺失，无法播放。`); updateButtons(); return; }
    try { await a.play(); setStatus(`正在播放 ${active === 'original' ? 'A 原音' : `B EQ（${selectedMode() === 'Headphone' ? '耳机' : '扬声器'}）`}。`); }
    catch (error) { setStatus(`播放失败：${error && error.message ? error.message : '浏览器无法解码或读取该音频文件。'}`); }
    updateButtons();
  }
  function updateTime() { const a = audios[active], dur = Number.isFinite(a.duration) ? a.duration : 0; if (!seekDrag) $('[data-seek]').value = dur > 0 ? String(Math.round(a.currentTime / dur * 1000)) : '0'; $('[data-time]').textContent = `${durationLabel(a.currentTime)} / ${durationLabel(dur)}`; }
  function refresh(force = false) {
    if (!valid) { $('[data-choice]').textContent = '音频清单尚未加载'; setStatus('找不到有效的 audio-manifest.js；请确认它与音频目录一起存在。'); syncDownload(); return; }
    const p = paths(), modeBoth = document.getElementById('mode').value === 'both';
    $('[data-mode-choice]').hidden = !modeBoth;
    $('[data-choice]').textContent = `当前预设：${current} · B：${p.mode === 'Headphone' ? '耳机' : '扬声器'}`;
    $('[data-extra-gain]').textContent = Number(manifest.extra_gain_db || 0).toFixed(3);
    const key = `${current}|${p.mode}`;
    if (!force && key === `${knownPreset}|${knownMode}`) return;
    knownPreset = current; knownMode = p.mode;
    for (const kind of ['original','eq']) {
      const a = audios[kind], path = p[kind];
      a.pause(); a.removeAttribute('src'); a.load();
      if (path) a.src = path;
    }
    updateButtons(); syncDownload(); updateTime();
    const missing = ['original','eq'].filter(k => !p[k]);
    setStatus(missing.length ? `素材缺失：${missing.map(k => k === 'original' ? '原音' : `${current} / ${p.mode}`).join('、')}。` : `已就绪；尚未播放。`);
  }
  host.querySelectorAll('[data-ab]').forEach(b => b.addEventListener('click', () => setActive(b.dataset.ab)));
  $('[data-toggle]').addEventListener('click', () => audios[active].paused ? playActive() : audios[active].pause());
  $('[data-volume]').addEventListener('input', e => Object.values(audios).forEach(a => a.volume = Number(e.target.value)));
  $('[data-loop]').addEventListener('change', e => Object.values(audios).forEach(a => a.loop = e.target.checked));
  $('[data-mode]').addEventListener('change', () => refresh(true));
  $('[data-params]').addEventListener('click', () => { document.getElementById('sr').value = '48000'; document.getElementById('gain').value = Number(manifest && manifest.extra_gain_db || 0); render(); setStatus('曲线显示已设为 48 kHz 与试听附加增益；音频本身不变。'); });
  $('[data-seek]').addEventListener('input', () => { seekDrag = true; const a = audios[active], d = Number.isFinite(a.duration) ? a.duration : 0; if (d) $('[data-time]').textContent = `${durationLabel(d * Number($('[data-seek]').value) / 1000)} / ${durationLabel(d)}`; });
  $('[data-seek]').addEventListener('change', () => { const d = audios[active].duration; if (Number.isFinite(d)) audios[active].currentTime = d * Number($('[data-seek]').value) / 1000; seekDrag = false; updateTime(); });
  for (const a of Object.values(audios)) {
    a.volume = Number($('[data-volume]').value);
    a.addEventListener('play', () => {
      if (a !== audios[active]) { a.pause(); return; }
      Object.values(audios).forEach(other => { if (other !== a) other.pause(); });
      updateButtons();
    });
    a.addEventListener('pause', updateButtons); a.addEventListener('ended', updateButtons);
    a.addEventListener('timeupdate', updateTime); a.addEventListener('loadedmetadata', updateTime);
    a.addEventListener('error', () => { setStatus(`音频加载失败：${a.getAttribute('src') || '文件路径不可用'}。请确认素材文件存在且格式受浏览器支持。`); updateButtons(); });
  }
  const originalRender = render;
  render = function(...args) { const result = originalRender.apply(this,args); refresh(); return result; };
  document.getElementById('mode').addEventListener('change', () => refresh(true));
  refresh(true);
})();
