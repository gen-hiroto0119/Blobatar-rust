import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const commit = "a7fd546ebede49d0a9fa638945b9e534489782a2";
if (!process.argv[2]) throw new Error("Pass the pinned upstream checkout path");
const root = resolve(process.argv[2]);
if ((await Bun.$`git -C ${root} rev-parse HEAD`.quiet()).text().trim() !== commit) throw new Error("Wrong upstream revision");
if ((await Bun.$`git -C ${root} status --porcelain --untracked-files=no`.quiet()).text().trim()) throw new Error("Upstream checkout has tracked modifications");
const load = (name: string) => import(pathToFileURL(`${root}/packages/blobatar/src/${name}.ts`).href);
const {_layout} = await load("blobatar");
const {idleAt, idleSeeds, idleTransforms} = await load("idle");
const expressions = await load("expression");

// The independent SVG-list parser from upstream's react-native-worklets.test.ts.
function matrix(list: string): number[] {
  let m = [1, 0, 0, 1, 0, 0];
  const mul = (n: number[]) => {
    m = [m[0]*n[0]+m[2]*n[1], m[1]*n[0]+m[3]*n[1],
      m[0]*n[2]+m[2]*n[3], m[1]*n[2]+m[3]*n[3],
      m[0]*n[4]+m[2]*n[5]+m[4], m[1]*n[4]+m[3]*n[5]+m[5]];
  };
  for (const [,fn,argv] of list.matchAll(/(\w+)\(([^)]*)\)/g)) {
    const a = argv.trim().split(/[\s,]+/).map(Number);
    if (fn === "translate") mul([1,0,0,1,a[0],a[1] ?? 0]);
    else if (fn === "scale") mul([a[0],0,0,a[1] ?? a[0],0,0]);
    else if (fn === "rotate") {
      const r = a[0]*Math.PI/180;
      mul([Math.cos(r),Math.sin(r),-Math.sin(r),Math.cos(r),0,0]);
    } else throw new Error(`Unhandled transform ${fn}`);
  }
  return m;
}
const names = ["idle","happy","sad","mad","surprised","wink","sleepy","smug","unsure","scared","love","shy","sick","thinking"];
const cases: any[] = [];
for (const seed of ["alice", "ひろと", "🍣"]) {
  const seeds = idleSeeds(seed);
  const times = [-1,0,449,450,451,1234,1000000,
    ...[0.972,0.986,1].flatMap(p=>[-0.01,0,0.01].map(d=>p*seeds.blink-seeds.blinkPhase+d))];
  for (const shape of [0.1,0.65,0.965]) for (const expression of names) {
    const options = {traits:{shape}};
    const layout = _layout(seed, options);
    const pose = expressions[expression].p;
    for (const time of times) for (const amplitude of [0,0.5,1]) {
      const idle = idleAt(seeds,time,amplitude,pose.shake);
      const t = idleTransforms(layout,pose,idle);
      const body = [t.root,t.breathe,t.bob].join(" ");
      cases.push({seed, options, expression, time, amplitude,
        transforms:{body:matrix(body),eyes:t.eye.map((eye: string,i: number)=>matrix([body,t.eyes,eye,t.glance[i]].join(" ")))}});
    }
  }
}
const bytes=JSON.stringify({meta:{commit,caseCount:cases.length},cases})+"\n";
const target=new URL("../crates/blobatar-motion/tests/fixtures/transforms-2.7.0.json",import.meta.url);
if(process.argv.includes("--write")) await Bun.write(target,bytes);
else if(await Bun.file(target).text()!==bytes) throw new Error("Fixture changed");
console.log(`${cases.length} transform matrix cases`);
