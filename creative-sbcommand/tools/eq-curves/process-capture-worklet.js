class ProcessCaptureProcessor extends AudioWorkletProcessor {
  constructor(){
    super();this.blocks=[];this.offset=0;this.phase=0;this.rate=sampleRate;
    this.dropped=0;this.underruns=0;this.invalid=0;this.reportFrames=0;this.active=true;this.primed=false;this.totalFrames=0;this.lastPacketAt=0;this.receivedFrames=0;this.inputPeak=0;
    this.port.onmessage=event=>{
      const message=event.data;
      if(message.type==='ready'){
        if(Number.isFinite(message.sample_rate)&&message.sample_rate>=8000&&message.sample_rate<=384000)this.rate=message.sample_rate;
        this.port.postMessage({type:'ready',contextRate:sampleRate,sourceRate:this.rate});
      }else if(message.type==='pcm'){
        if(!(message.buffer instanceof ArrayBuffer)||message.buffer.byteLength%8){this.invalid++;return;}
        const samples=new Float32Array(message.buffer);
        if(samples.length){
          this.blocks.push(samples);this.receivedFrames+=samples.length/2;this.lastPacketAt=this.totalFrames;this.inputPeak=0;
          for(const value of samples)if(Number.isFinite(value))this.inputPeak=Math.max(this.inputPeak,Math.abs(value));
        }
        const limit=Math.ceil(this.rate*.5);
        while((this.queuedFrames()>limit||this.blocks.length>256)&&this.blocks.length){
          const count=this.blocks[0].length/2-this.offset;this.blocks.shift();this.offset=0;this.phase=0;this.dropped+=count;
        }
      }else if(message.type==='clear'||message.type==='stop'){
        this.blocks=[];this.offset=0;this.phase=0;this.primed=false;if(message.type==='stop')this.active=false;
      }
    };
  }
  queuedFrames(){let count=-this.offset;for(const block of this.blocks)count+=block.length/2;return Math.max(0,count);}
  at(channel,delta){
    let position=this.offset+delta;
    for(const block of this.blocks){const frames=block.length/2;if(position<frames){const value=block[position*2+channel];return Number.isFinite(value)?value:0;}position-=frames;}
    return 0;
  }
  consume(count){
    this.offset+=count;
    while(this.blocks.length&&this.offset>=this.blocks[0].length/2){this.offset-=this.blocks[0].length/2;this.blocks.shift();}
    if(!this.blocks.length){this.offset=0;this.phase=0;}
  }
  process(_inputs,outputs){
    const out=outputs[0];if(!this.active)return false;if(!out?.length)return true;
    const step=this.rate/sampleRate;
    if(!this.primed&&this.queuedFrames()>=Math.ceil(this.rate*.03))this.primed=true;
    for(let frame=0;frame<out[0].length;frame++){
      const needed=Math.max(2,Math.floor(this.phase+step));
      if(!this.primed||this.queuedFrames()<needed){
        this.primed=false;
        for(let channel=0;channel<out.length;channel++)out[channel][frame]=0;
        this.underruns++;continue;
      }
      for(let channel=0;channel<out.length;channel++){
        const a=this.at(Math.min(channel,1),0),b=this.at(Math.min(channel,1),1);
        out[channel][frame]=a+(b-a)*this.phase;
      }
      this.phase+=step;const consumed=Math.floor(this.phase);this.phase-=consumed;this.consume(consumed);
    }
    this.totalFrames+=out[0].length;this.reportFrames+=out[0].length;
    if(this.reportFrames>=sampleRate/10){
      this.reportFrames=0;this.port.postMessage({type:'buffer',queued:this.blocks.length,queued_frames:this.queuedFrames(),sourceRate:this.rate,contextRate:sampleRate,dropped:this.dropped,underruns:this.underruns,invalid:this.invalid,received_frames:this.receivedFrames,input_peak:this.inputPeak,packet_age_frames:this.totalFrames-this.lastPacketAt});
    }
    return true;
  }
}
registerProcessor('process-capture',ProcessCaptureProcessor);
