import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const commit = "a7fd546ebede49d0a9fa638945b9e534489782a2";
if (!process.argv[2]) throw new Error("Pass pinned upstream checkout [--write]");
const root = resolve(process.argv[2]);
if ((await Bun.$`git -C ${root} rev-parse HEAD`.quiet()).text().trim() !== commit) throw new Error("Wrong revision");
if ((await Bun.$`git -C ${root} status --porcelain --untracked-files=no`.quiet()).text().trim()) throw new Error("Tracked upstream changes");
Bun.plugin({ name: "pinned-site-aliases", setup(build) {
  build.onResolve({ filter: /^@\// }, ({ path }) => ({ path: `${root}/apps/site/src/${path.slice(2)}.ts` }));
  build.onResolve({ filter: /^blobatar$/ }, () => ({ path: `${root}/packages/blobatar/src/index.ts` }));
} });
const load = (path: string) => import(pathToFileURL(`${root}/${path}.ts`).href);
const axes = await load("apps/site/src/editor/axes");
const { resolved, blobLayout } = await load("apps/site/src/editor/resolved");
const { snippet } = await load("apps/site/src/editor/snippet");
const { traits: reader } = await load("packages/blobatar/src/traits");

const toggles: any[] = [];
for (const [key, choices, toggle] of [["shape", axes.SHAPES, axes.toggleShape], ["tone", axes.TONES, axes.toggleTone]] as const) {
  for (let mask = 0; mask < 1 << choices.length; mask++) {
    const chosen = choices.filter((_: any, i: number) => mask & (1 << i)).map((choice: any) => choice.at);
    for (const { at } of choices) {
      const next = toggle(chosen, at);
      toggles.push({ key, chosen, at, next, pin: axes.narrowPin(next) ?? null });
    }
  }
}
const states: any[] = [];
for (const name of ["", "alain00", "ひろと", "🦀", "cafe\u0301", "İstanbul", " a\u00a0b "]) {
  for (const shape of [undefined, ...axes.SHAPES.map((shape: any) => shape.at), axes.SHAPES.map((shape: any) => shape.at), [0.11, 0.965], [0.2, 0.8], []]) {
    for (const extra of [{}, { "eye.rx": 1, "eye.gap": 1, "gaze.x": 1, "gaze.y": 1 }, { "eye.rx": .999, "eye.ratio": 0, "eye.scale": 1, "eye.gap": .9 }]) {
      const pinned = { ...(shape === undefined ? {} : { shape }), ...extra };
      const layout = blobLayout(name || "blobatar", pinned);
      const t = reader(name, true, pinned);
      const shapes = axes.candidates(pinned.shape, layout.shape);
      states.push({ name, pinned, shape: layout.shape, shapes, applicable: axes.AXES.filter((axis: any) => axes.applies(axis, shapes)).map((axis: any) => axis.key), ghosts: resolved(layout, t), pins: Object.fromEntries(axes.AXES.map((axis: any) => [axis.key, axes.round3(t(axis.key))])) });
    }
  }
}
const snippets: any[] = [];
for (const api of ["react", "vue", "svelte", "solid", "preact", "react-native", "string", "http"]) {
  for (const name of ["", "ひろと🦀", "say \"hi\"\\world", "a & b < c", "line\nbreak"]) {
    for (const pinned of [{}, { shape: .11 }, { hue: .123, "eye.gap": .501, shape: [.11, .965], tone: [.1, .965] }, { zed: .1, "body.r": .2, alpha: .3, "odd'\\key": .4 }, { hue: .333, tone: .865 }, { shape: [], "eye.rx": .125 }]) {
      for (const motion of [false, "hover", "always"]) snippets.push({ api, name, pinned, motion, output: snippet({ api, name, pinned, motion }) });
    }
  }
}
const bands: any[] = [];
for (const count of [2, 3, 4]) {
  for (const value of [-1, 0, .2499, .25, .3333, .5, .75, .9999, 1, 2]) bands.push({ count, value, index: axes.bandIndex(value, count) });
}
const fixture = { meta: { commit, oracle: "Pinned editor axes, resolved geometry, and snippet functions; all selection is retained, not auto" }, axes: axes.AXES.map((axis: any) => ({ ...axis, when: axis.when ?? [], bands: axis.bands ?? null })), shapes: axes.SHAPES, tones: axes.TONES, bands, toggles, states, snippets };
const target = new URL("../crates/blobatar-ui/tests/fixtures/editor-2.7.0.json", import.meta.url);
const bytes = JSON.stringify(fixture) + "\n";
if (process.argv.includes("--write")) await Bun.write(target, bytes);
else if (await Bun.file(target).text() !== bytes) throw new Error("Editor fixture changed");
console.log(`${toggles.length} toggles, ${states.length} editor states, ${snippets.length} snippets`);
