import {resolve} from "node:path";
import {pathToFileURL} from "node:url";
import {matrix} from "./svg-matrix";

const commit="a7fd546ebede49d0a9fa638945b9e534489782a2";
if(!process.argv[2])throw new Error("Pass the pinned upstream checkout [--write]");
const root=resolve(process.argv[2]);
if((await Bun.$`git -C ${root} rev-parse HEAD`.quiet()).text().trim()!==commit)throw new Error("Wrong revision");
if((await Bun.$`git -C ${root} status --porcelain --untracked-files=no`.quiet()).text().trim())throw new Error("Tracked upstream changes");
const load=(name:string)=>import(pathToFileURL(`${root}/packages/blobatar/src/${name}.ts`).href);
const {_layout}=await load("blobatar");
const {idleAt,idleSeeds,idleTransforms}=await load("idle");
const expressions=await load("expression");
const driver=await Bun.file(new URL("../crates/blobatar-motion/tests/fixtures/driver-2.7.0.json",import.meta.url)).json();
if(driver.meta.commit!==commit)throw new Error("Wrong driver fixture");
const frames=driver.cases[0].actions.map((a:any)=>a.output);
const gazes=[frames[0],frames.find((f:any)=>f.hold>0&&f.hold<1),frames.find((f:any)=>f.hold===1&&f.direction.x!==0),frames.find((f:any)=>f.hold===1&&f.direction.x===0&&f.direction.y===0)];
if(gazes.some(f=>!f))throw new Error("Missing frozen gaze states");

// Fold gaze.css channels into upstream's already-quantized SVG pose list.
// This is an adapter-composition oracle, not a browser computed-style capture.
function applyGaze(list:string,g:any):string {
  const parts=[...list.matchAll(/(\w+)\(([^)]*)\)/g)].map(([,name,argv])=>({name,args:argv.trim().split(/[\s,]+/).map(Number)}));
  if(parts.map(p=>p.name).join(",")!=="translate,rotate,scale,rotate,translate")throw new Error("Pose list changed");
  parts[0].args[0]+=g.dx;parts[0].args[1]+=g.dy;
  parts[1].args[0]+=g.t;
  parts[2].args[0]*=g.sx;parts[2].args[1]*=g.sy;
  return parts.map(p=>`${p.name}(${p.args.join(" ")})`).join(" ");
}
const cases:any[]=[];
const seed="ひろと";
for(const shape of [0.1,0.965])for(const expression of ["idle","happy","sad","mad","surprised","wink","sleepy","smug","unsure","scared","love","shy","sick","thinking"]){
  const options={traits:{shape}}, layout=_layout(seed,options), pose=expressions[expression].p;
  for(const time of [0,1234,3000])for(const amplitude of [0,1])for(const gaze of gazes){
    const seeds=idleSeeds(seed);
    for(const key of ["lookX","lookY","lookMX","lookMY"])seeds[key]*=1-gaze.hold;
    const idle=idleAt(seeds,time,amplitude,pose.shake);
    const t=idleTransforms(layout,pose,idle);
    const hover=amplitude*0.65;
    const lift=`translate(50 ${50-1.5*hover}) scale(${1+0.04*hover}) translate(-50 -50)`;
    const body=[t.root,lift,t.breathe,t.bob].join(" ");
    cases.push({seed,options,expression,time,amplitude,hover,gaze,idle,transforms:{body:matrix(body),eyes:t.eye.map((eye:string,i:number)=>matrix([body,t.eyes,applyGaze(eye,gaze.eyes[i]),t.glance[i]].join(" ")))}});
  }
}
const fixture={meta:{commit,caseCount:cases.length,oracle:"Pinned idleTransforms SVG lists + gaze.css channel folding; not native/browser measurement or computed-style parity"},cases};
const target=new URL("../crates/blobatar-motion/tests/fixtures/gaze-transforms-2.7.0.json",import.meta.url);
const bytes=JSON.stringify(fixture)+"\n";
if(process.argv.includes("--write"))await Bun.write(target,bytes);
else if(await Bun.file(target).text()!==bytes)throw new Error("Gaze transform fixture changed");
console.log(`${cases.length} gaze/pose/hover/blink composition cases`);
