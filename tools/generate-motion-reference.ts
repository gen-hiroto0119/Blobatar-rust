import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const commit = "a7fd546ebede49d0a9fa638945b9e534489782a2";
const root = resolve(process.argv[2] ?? "");
if (!process.argv[2]) throw new Error("Pass the pinned upstream checkout path");
if ((await Bun.$`git -C ${root} rev-parse HEAD`.quiet()).text().trim() !== commit) {
  throw new Error("Wrong upstream revision");
}
if ((await Bun.$`git -C ${root} status --porcelain --untracked-files=no`.quiet()).text().trim()) {
  throw new Error("Upstream checkout has tracked modifications");
}
const load = (name: string) => import(pathToFileURL(`${root}/packages/blobatar/src/${name}.ts`).href);
const expressions = await load("expression");
const { lerpPose } = await load("morph");
const { idleSeeds, idleAt } = await load("idle");
const { project, step, threshold, pursuit } = await load("gaze");
const names = ["idle", "happy", "sad", "mad", "surprised", "wink", "sleepy", "smug", "unsure", "scared", "love", "shy", "sick", "thinking"];
const poses = names.map(name => ({ name, pose: expressions[name].p }));
const morphs = names.flatMap(from => names.flatMap(to => [0, 0.25, 0.5, 0.9, 1].map(t => ({
  from, to, t, pose: lerpPose(expressions[from].p, expressions[to].p, t),
}))));
const idle: any[] = [];
for (const name of ["alice", "ひろと", "e\u0301", "🍣", ""] ) {
  const seeds = idleSeeds(name);
  // Sample just before/on/after every blink boundary, plus long and negative time.
  const times = [-3500, -1, 0, 16, 450, 900, 2800, 3400, 1000000,
    ...[0.972, 0.986, 1].flatMap(p => [-0.01, 0, 0.01].map(d => p * seeds.blink - seeds.blinkPhase + d))];
  for (const time of times) for (const amplitude of [0, 0.5, 1]) for (const shake of [0, 0.55]) {
    idle.push({ name, seeds, time, amplitude, shake, frame: idleAt(seeds, time, amplitude, shake) });
  }
}
const projections: any[] = [];
for (const mark of [{x: 0, y: 0}, {x: -0.3, y: 0.1}, {x: 0.5, y: -0.4}, {x: 1, y: 0}, {x: 2, y: 2}]) {
  for (const yaw of [-Math.PI, -1, 0, 0.6, Math.PI]) for (const pitch of [-1, 0, 1]) {
    projections.push({ mark, yaw, pitch, result: project(mark, yaw, pitch) });
  }
}
const steps: any[] = [];
for (const [x, y] of [[0, 0], [1, 0], [-1, -1]]) {
  for (const [dx, dy] of [[0, 0], [1, 1], [200, -200], [-1000, 0]]) {
    for (const radius of [0, 1, 100]) for (const dt of [0, 16, 64]) {
      const input = { x, y, dx, dy, radius, k: pursuit(dt), gain: 1, snap: 1.6 };
      steps.push({ input, result: step(input) });
    }
  }
}
const thresholds = [0, 16, 100, 512].flatMap(width => [0, 0.1, 1, 10].map(travel => ({width, travel, value: threshold(width, travel)})));
const pursuits = [0, 1, 16, 64, 1000].flatMap(dt => [0, 110, 1000].map(settle => ({dt, settle, value: pursuit(dt, settle)})));
const bytes = JSON.stringify({meta: {commit}, poses, morphs, idle, projections, steps, thresholds, pursuits}) + "\n";
const target = new URL("../crates/blobatar-motion/tests/fixtures/motion-2.7.0.json", import.meta.url);
if (process.argv.includes("--write")) await Bun.write(target, bytes);
else if (await Bun.file(target).text() !== bytes) throw new Error("Fixture changed");
console.log(`${poses.length} poses, ${morphs.length} morphs, ${idle.length} idle frames, ${projections.length} projections, ${steps.length} pursuit steps`);
