import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const commit = "a7fd546ebede49d0a9fa638945b9e534489782a2";
if (!process.argv[2]) throw new Error("Pass the pinned upstream checkout path");
const root = resolve(process.argv[2]);
if ((await Bun.$`git -C ${root} rev-parse HEAD`.quiet()).text().trim() !== commit) {
  throw new Error("Wrong upstream revision");
}
if ((await Bun.$`git -C ${root} status --porcelain --untracked-files=no`.quiet()).text().trim()) {
  throw new Error("Upstream checkout has tracked modifications");
}
const load = (name: string) => import(pathToFileURL(`${root}/packages/blobatar/src/${name}.ts`).href);
const { blobatar, _layout } = await load("blobatar");
const { blobatarUri } = await load("uri");
const expressions = await load("expression");
const names = ["idle", "happy", "sad", "mad", "surprised", "wink", "sleepy", "smug", "unsure", "scared", "love", "shy", "sick", "thinking"];
const shapes = [0.1, 0.35, 0.55, 0.65, 0.75, 0.82, 0.89, 0.93, 0.965, 0.99];
const cases: any[] = [];
for (const seed of ["alice", "ひろと", "e\u0301", "🍣"]) {
  for (const shape of shapes) for (const tone of [0.1, 0.25, 0.5, 0.7, 0.85, 0.99]) {
    for (const expression of names) {
      const options = { traits: {shape}, tone, background: "squircle", expression };
      const referenceOptions = {...options, expression: expressions[expression]};
      const {eyes, palette} = _layout(seed, referenceOptions);
      cases.push({seed, options, eyes, palette, svg: blobatar(seed, referenceOptions)});
    }
  }
}
const overrides: any[] = [];
for (const name of ["mad", "love", "shy", "sick"]) {
  for (const palette of [{head: "#eeeeee", eye: "#111111"}, {head: "#223355", eye: "#ffffff"}]) {
    const options = {palette, expression: name, title: "日本語 🍣 <&>"};
    const referenceOptions = {...options, expression: expressions[name]};
    overrides.push({seed: "override", options, svg: blobatar("override", referenceOptions), uri: blobatarUri("override", referenceOptions)});
  }
}
const bytes = JSON.stringify({meta: {commit, caseCount: cases.length}, cases, overrides}) + "\n";
const target = new URL("../crates/blobatar-core/tests/fixtures/expressions-2.7.0.json", import.meta.url);
if (process.argv.includes("--write")) await Bun.write(target, bytes);
else if (await Bun.file(target).text() !== bytes) throw new Error("Fixture changed");
console.log(`${cases.length} shape/expression/seed/tone cases, ${overrides.length} palette overrides`);
