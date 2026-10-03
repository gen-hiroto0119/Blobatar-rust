import {resolve} from "node:path";
import {pathToFileURL} from "node:url";

const commit="a7fd546ebede49d0a9fa638945b9e534489782a2";
if(!process.argv[2]) throw new Error("Pass the pinned upstream checkout [--write]; Chrome must expose CDP_URL (default http://localhost:29229)");
const root=resolve(process.argv[2]);
if((await Bun.$`git -C ${root} rev-parse HEAD`.quiet()).text().trim()!==commit) throw new Error("Wrong upstream revision");
if((await Bun.$`git -C ${root} status --porcelain --untracked-files=no`.quiet()).text().trim()) throw new Error("Tracked upstream changes");
const {_parts}=await import(pathToFileURL(`${root}/packages/blobatar/src/blobatar.ts`).href);
const bundle=await Bun.build({entrypoints:[`${root}/packages/blobatar/src/gaze.ts`],target:"browser",minify:true});
if(!bundle.success)throw new Error(String(bundle.logs));
const moduleUrl=`data:text/javascript;base64,${Buffer.from(await bundle.outputs[0].text()).toString("base64")}`;
const cdp=process.env.CDP_URL || "http://localhost:29229";
const version=await (await fetch(`${cdp}/json/version`)).json();
const tab=await (await fetch(`${cdp}/json/new?about:blank`,{method:"PUT"})).json();
const ws=new WebSocket(tab.webSocketDebuggerUrl);
await new Promise(r=>ws.addEventListener("open",r,{once:true}));
let id=0;
const pending=new Map();
ws.onmessage=e=>{const r=JSON.parse(e.data);if(r.id){const p=pending.get(r.id);pending.delete(r.id);r.error?p.reject(r.error):p.resolve(r.result);}};
const call=(method,params={})=>new Promise((resolve,reject)=>{const n=++id;pending.set(n,{resolve,reject});ws.send(JSON.stringify({id:n,method,params}));});
const evaluate=async expression=>{
  const result=await call("Runtime.evaluate",{expression,awaitPromise:true,returnByValue:true});
  if(result.exceptionDetails)throw new Error(JSON.stringify(result.exceptionDetails));
  return result.result.value;
};
try {
  await evaluate(`import(${JSON.stringify(moduleUrl)}).then(m=>globalThis.survey=m.survey)`);
  const cases=[];
  for(const seed of ["Alice","ひろと","😀","e\u0301"]) for(const shape of [0.1,0.35,0.55,0.65,0.75,0.82,0.89,0.93,0.965,0.99]) for(const extremes of [null,0,1]) {
    const traits = extremes===null ? {shape} : {shape,"eye.gap":extremes,"eye.rx":extremes,"eye.ratio":extremes,"eye.scale":extremes,"eye.stretch":extremes,"eye.lean":extremes};
    const options={traits};
    const parts=_parts(seed,{...options,animate:"always"});
    const html=`<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="100" height="100">${parts.inner}</svg>`;
    const measured=await evaluate(`(()=>{
      document.body.innerHTML=${JSON.stringify(html)};
      const head=document.querySelector('.mo-bob > g:not(.mo-eyes)');
      const box=e=>{const{x,y,width,height}=e.getBBox();return{x,y,width,height};};
      const samples=[];
      for(const element of head.querySelectorAll('path,circle')) {
        const original=element.isPointInFill.bind(element);
        element.isPointInFill=point=>{
          const hit=original(point);
          const last=samples.at(-1);
          if(last&&last.x===point.x&&last.y===point.y)last.hit ||= hit;
          else samples.push({x:point.x,y:point.y,hit});
          return hit;
        };
      }
      const face=globalThis.survey(document.querySelector('svg'));
      return {head:box(head),eyes:[...document.querySelectorAll('.mo-eye')].map(box),samples,face};
    })()`);
    if(!measured.face)throw new Error(`No face: ${seed},${shape},${extremes}`);
    cases.push({seed,options,...measured});
  }
  const fixture={meta:{commit,browser:version.Browser,caseCount:cases.length},cases};
  const path=resolve(import.meta.dir,"../crates/blobatar-motion/tests/fixtures/survey-2.7.0.json");
  if(process.argv.includes("--write"))await Bun.write(path,JSON.stringify(fixture,null,2)+"\n");
  else {
    const previous=await Bun.file(path).json();
    if(JSON.stringify(previous.cases)!==JSON.stringify(cases))throw new Error("Survey fixture differs; compare browser version and geometry before accepting");
  }
  console.log(`${cases.length} browser survey cases (${version.Browser})`);
} finally {ws.close();await fetch(`${cdp}/json/close/${tab.id}`);}
