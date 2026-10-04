import {resolve} from "node:path";

const commit = "a7fd546ebede49d0a9fa638945b9e534489782a2";
if (!process.argv[2]) throw new Error("Pass the pinned upstream checkout [--write]");
const root = resolve(process.argv[2]);
if ((await Bun.$`git -C ${root} rev-parse HEAD`.quiet()).text().trim() !== commit) throw new Error("Wrong upstream revision");
if ((await Bun.$`git -C ${root} status --porcelain --untracked-files=no`.quiet()).text().trim()) throw new Error("Tracked upstream changes");
const source = await Bun.file(`${root}/packages/blobatar/src/gaze.ts`).text();
const seam = "  };\n\n  const wake = () => {";
if (source.split(seam).length !== 2) throw new Error("Driver instrumentation seam changed");
// Observe private f64 state after a frame, without replacing any arithmetic.
const instrumented = source.replace(seam, "    globalThis.__gazeProbe([x, y, h]);\n" + seam);
const js = new Bun.Transpiler({loader: "ts"}).transformSync(instrumented);
const {gaze} = await import(`data:text/javascript;base64,${Buffer.from(js).toString("base64")}`);
const survey = await Bun.file(resolve(import.meta.dir, "../crates/blobatar-motion/tests/fixtures/survey-2.7.0.json")).json();

class Style {
  values = new Map<string,string>();
  setProperty(k: string, v: string) { this.values.set(k,v); }
  removeProperty(k: string) { this.values.delete(k); }
  number(k: string, fallback = 0) { return Number(this.values.get(k) ?? fallback); }
}
const rect = (b: any) => ({...b, left:b.x, top:b.y});
const cases: any[] = [];
for (const size of [24,100,256]) for (const travel of [2.5,15]) for (const index of [0,27]) {
  let geometry = survey.cases[index];
  let sample = 0;
  let viewport = {x:100,y:80,width:size,height:size};
  let watchedBounds = {x:360,y:200,width:80,height:32};
  const watched = {getBoundingClientRect:()=>rect(watchedBounds)};
  let layer = {style:new Style(), isConnected:true};
  const eyeNodes = () => geometry.eyes.map((b:any)=>({getBBox:()=>b}));
  const head = {
    getBBox:()=>{sample=0;return geometry.head;},
    querySelectorAll:()=>[{isPointInFill:(p:any)=>{
      const expected=geometry.samples[sample++];
      if (!expected || Math.abs(p.x-expected.x)>1e-9 || Math.abs(p.y-expected.y)>1e-9) throw new Error("Unexpected survey query");
      return expected.hit;
    }}],
  };
  const element = {
    style:new Style(),
    querySelector:(s:string)=>s===".mo-eyes"?layer:head,
    querySelectorAll:()=>eyeNodes(),
    getBoundingClientRect:()=>rect(viewport),
  };
  const events = new Map<string,Function>();
  const media = (matches:boolean)=>({matches, listener:null as any, addEventListener(_:any,f:any){this.listener=f;},removeEventListener(){this.listener=null;}});
  const fine=media(true), still=media(false);
  let sequence=0, raw:any=null;
  const frames=new Map<number,Function>();
  Object.assign(globalThis,{
    __gazeProbe:(v:any)=>{raw=v;},
    DOMPoint:class {constructor(public x:number,public y:number){}},
    ResizeObserver:class {observe(){} unobserve(){} disconnect(){}},
    matchMedia:(s:string)=>s.includes("prefers-reduced")?still:fine,
    addEventListener:(s:string,f:Function)=>events.set(s,f),
    removeEventListener:(s:string)=>events.delete(s),
    getComputedStyle:()=>({getPropertyValue:()=>String(travel)}),
    requestAnimationFrame:(f:Function)=>{frames.set(++sequence,f);return sequence;},
    cancelAnimationFrame:(n:number)=>frames.delete(n),
  });
  const driver=gaze(element);
  if(sample!==geometry.samples.length)throw new Error("Initial survey did not consume its fixture");
  const actions:any[]=[];
  let time=0;
  const act=(action:any)=>{
    raw=null;
    switch(action.kind){
      case "tick": {
        const pending=[...frames.values()];frames.clear();
        for(const f of pending)f(action.time);
        if(sample!==geometry.samples.length)throw new Error("Survey failed inside the driver");
        break;
      }
      case "pointer":events.get("pointermove")?.({clientX:action.x,clientY:action.y});break;
      case "leave":events.get("pointerleave")?.();break;
      case "target":driver.lookAt(action.value==="element"?watched:action.value);break;
      case "measure":viewport=action.bounds;watchedBounds=action.watched;driver.remeasure();break;
      case "replace":layer.isConnected=false;layer={style:new Style(),isConnected:true};geometry=survey.cases[action.index];driver.remeasure();break;
      case "enabled":still.matches=!action.value;still.listener?.();break;
      case "stop":driver.stop();break;
      default:throw new Error(action.kind);
    }
    const output={direction:{x:layer.style.number("--mo-track-x"),y:layer.style.number("--mo-track-y")},hold:element.style.number("--mo-track-hold"),eyes:geometry.eyes.map((_:any,i:number)=>Object.fromEntries(["dx","dy","sx","sy","t"].map(k=>[k,layer.style.number(`--mo-gz-${k}${i+1}`,k.startsWith("s")?1:0)])))};
    actions.push({action,raw,scheduled:frames.size>0,output});
  };
  const ticks=(n:number,dt=16)=>{for(let i=0;i<n;i++){act({kind:"tick",time});time+=dt;}};
  ticks(3);
  act({kind:"target",value:"pointer"});ticks(3);
  act({kind:"pointer",x:250,y:140});ticks(5);
  act({kind:"pointer",x:50,y:140});ticks(5);
  act({kind:"target",value:{x:100+size/2+0.02*size,y:80+size/2}});ticks(4);
  act({kind:"target",value:"element"});ticks(4);
  act({kind:"measure",bounds:{x:120,y:70,width:size,height:size/2},watched:{x:50,y:20,width:50,height:40}});ticks(4);
  act({kind:"leave"});ticks(2);
  act({kind:"replace",index:index===0?27:0});ticks(70);
  act({kind:"target",value:{x:1000,y:70+size/4}});ticks(2);
  act({kind:"target",value:"rest"});ticks(70);
  act({kind:"target",value:null});ticks(70);
  act({kind:"target",value:"pointer"});ticks(4);
  time+=10_000;ticks(3);
  act({kind:"enabled",value:false});
  act({kind:"pointer",x:900,y:800});ticks(3);
  act({kind:"enabled",value:true});ticks(5);
  act({kind:"stop"});ticks(3);
  act({kind:"enabled",value:false});act({kind:"enabled",value:true});
  act({kind:"target",value:{x:100,y:100}});ticks(3);
  cases.push({index,size,travel,actions});
}
const rounding=[0,-0,0.0625,-0.0625,1.005,-1.005,1.2345,-1.2345,0.00005,-0.00005,Number.MIN_VALUE,Number.MAX_VALUE].flatMap(value=>[0,1,2,3,4].map(digits=>({value,digits,result:Number(value.toFixed(digits))})));
const fixture={meta:{commit,instrumentation:"Observer only after upstream frame; DOM layout supplied by frozen survey fixture; no browser scheduling or native adapter claim",caseCount:cases.length},rounding,cases};
const target=resolve(import.meta.dir,"../crates/blobatar-motion/tests/fixtures/driver-2.7.0.json");
const bytes=JSON.stringify(fixture)+"\n";
if(process.argv.includes("--write"))await Bun.write(target,bytes);
else if(await Bun.file(target).text()!==bytes)throw new Error("Driver fixture changed");
console.log(`${cases.length} driver sequences, ${cases.reduce((n,c)=>n+c.actions.length,0)} event/frame snapshots`);
