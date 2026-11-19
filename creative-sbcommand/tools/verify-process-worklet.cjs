'use strict';
const fs=require('fs'),path=require('path'),vm=require('vm');
const root=path.resolve(__dirname,'..'),source=fs.readFileSync(path.join(root,'tools/eq-curves/process-capture-worklet.js'),'utf8');
const checks=[];
function check(name,condition,details){checks.push({name,passed:!!condition,details});if(!condition)throw Error(name)}
function make(rate=48000){let Constructor;class Processor{constructor(){this.port={messages:[],postMessage(m){this.messages.push(m)},onmessage:null}}}const context=vm.createContext({AudioWorkletProcessor:Processor,registerProcessor(_name,C){Constructor=C},sampleRate:rate,ArrayBuffer,Float32Array,Math,Number});vm.runInContext(source,context);return new Constructor()}
function post(node,data){node.port.onmessage({data})}
function pcm(node,frames,value=.1){const a=new Float32Array(frames*2);for(let i=0;i<frames;i++){a[i*2]=value;a[i*2+1]=-value}post(node,{type:'pcm',buffer:a.buffer})}
function block(node){const out=[new Float32Array(128),new Float32Array(128)];node.process([], [out]);return out}
let result;
try{
 const p=make();for(let i=0;i<20;i++)block(p);
 check('empty input does not advance source cursor',p.offset===0&&p.phase===0&&p.queuedFrames()===0);
 post(p,{type:'ready',sample_rate:48000});pcm(p,8192);
 let out=block(p);check('startup recovers after waiting for PCM',Math.abs(out[0][100]-.1)<1e-7&&Math.abs(out[1][100]+.1)<1e-7);
 for(let i=0;i<100;i++)block(p);const cursor=p.offset;
 for(let i=0;i<20;i++)block(p);check('underflow does not accumulate an unpayable cursor',p.offset===cursor);
 pcm(p,8192,.2);out=block(p);check('fresh PCM recovers after underflow',out[0].some(v=>v>.15));
 for(let i=0;i<100;i++)pcm(p,2048,.1);check('buffer bounded at half a second',p.queuedFrames()<=24000&&p.blocks.length<=256&&p.dropped>0,{queued:p.queuedFrames(),dropped:p.dropped});
 const reports=p.port.messages.length;for(let i=0;i<100;i++)block(p);check('statistics are throttled',p.port.messages.length-reports<=4,{newReports:p.port.messages.length-reports});
 post(p,{type:'clear'});const bad=new Float32Array(4096);bad.fill(NaN);post(p,{type:'pcm',buffer:bad.buffer});out=block(p);check('nonfinite input cannot poison output',out.every(channel=>channel.every(Number.isFinite)));
 post(p,{type:'clear'});post(p,{type:'pcm',buffer:new ArrayBuffer(3)});check('incomplete stereo frames rejected',p.invalid===1&&p.queuedFrames()===0);
 for(const [inputRate,outputRate] of [[44100,48000],[48000,44100],[96000,48000]]){
  const node=make(outputRate);post(node,{type:'ready',sample_rate:inputRate});const count=16000,input=new Float32Array(count*2);
  for(let i=0;i<count;i++){input[i*2]=.2*Math.sin(2*Math.PI*1000*i/inputRate);input[i*2+1]=-input[i*2]}
  post(node,{type:'pcm',buffer:input.buffer});let maximum=0;const step=inputRate/outputRate;
  for(let n=0;n<4096/128;n++){
   const y=block(node);
   for(let i=0;i<128;i++){const at=(n*128+i)*step,k=Math.floor(at),fraction=at-k,expected=input[k*2]+(input[(k+1)*2]-input[k*2])*fraction;maximum=Math.max(maximum,Math.abs(y[0][i]-expected),Math.abs(y[1][i]+expected))}
  }
  check(`linear SRC ${inputRate} to ${outputRate} preserves timing and stereo`,maximum<1e-6,{maxAbsError:maximum});
 }
 post(p,{type:'stop'});check('stop terminates worklet processing',p.process([],[[new Float32Array(128),new Float32Array(128)]])===false);
 result={result:'PASS',checks};
}catch(error){result={result:'FAIL',error:String(error),checks}}
fs.writeFileSync(path.join(root,'tools/eq-curves/audio/worklet-validation.json'),JSON.stringify(result,null,2));console.log(JSON.stringify(result,null,2));if(result.result!=='PASS')process.exitCode=1;
