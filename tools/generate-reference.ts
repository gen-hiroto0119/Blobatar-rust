import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const commit = "a7fd546ebede49d0a9fa638945b9e534489782a2";
const reference = process.argv[2];
if (!reference) throw new Error("Usage: bun tools/generate-reference.ts UPSTREAM_CHECKOUT [--write]");
const root = resolve(reference);
const sha = (await Bun.$`git -C ${root} rev-parse HEAD`.quiet()).text().trim();
if (sha !== commit) throw new Error(`Expected ${commit}, got ${sha}`);
if ((await Bun.$`git -C ${root} status --porcelain --untracked-files=no`.quiet()).text().trim()) {
  throw new Error("Reference checkout has tracked modifications");
}
const load = (file: string) => import(pathToFileURL(`${root}/packages/blobatar/${file}`).href);
const { seedState, stream, normalizeSeed } = await load("src/hash.ts");
const { traits } = await load("src/traits.ts");
const { layout, blobatar } = await load("src/blob.ts");
const { resolve: resolveOptions } = await load("src/render.ts");
const { superellipse } = await load("src/shape.ts");
const { palette, ramp } = await load("src/color.ts");
const { blobatarUri } = await load("src/uri.ts");
const { BLOB_KEYS } = await load("test/keys.ts");
const legacy = await Bun.file(`${root}/packages/flutter/test/fixtures/reference-vectors.json`).json();
const seeds = [...new Set([...legacy.hash.map((v: any) => v.seed),
  "\ufeff ALICE \ufeff", "\u0085ALICE\u0085", "ΟΣ", "ΟΣΑ", "İ", "e\u0301", "🍣🦀", "ひろと", "\u2007Alice\u202f",
])];
function snapshot(seed: string, options: any, render = false) {
  const { t, palette } = resolveOptions(seed, options);
  const l = layout(t);
  return { seed, options, shape: l.shape, body: l.body, face: l.face, eyes: l.eyes,
    petals: l.petals, extra: l.extra,
    bodyPath: l.draw ? l.draw(l.body) : superellipse(l.body),
    eyePaths: l.eyes.map(superellipse), palette,
    ...(render ? { svg: blobatar(seed, options), uri: blobatarUri(seed, options) } : {}),
  };
}
const cases = legacy.cases.map((c: any) => snapshot(c.seed, c.options));
const shapes = [0.1, 0.35, 0.55, 0.65, 0.75, 0.82, 0.89, 0.93, 0.965, 0.99];
for (const shape of shapes) {
  for (const key of BLOB_KEYS) {
    for (const value of [0, 1, -1, [], [0.12, 0.96]]) {
      cases.push(snapshot(`axis-${key}`, { traits: { shape, [key]: value } }));
    }
  }
}
for (const seed of seeds) {
  for (const normalize of [true, false]) cases.push(snapshot(seed, { normalize }, true));
}
for (const shape of shapes) {
  for (const background of [false, true, "circle", "square", "squircle"]) {
    cases.push(snapshot("options<&>\"'", {
      traits: { shape, hue: 0.9, tone: 0.9 }, hue: 42, tone: 1,
      background, size: 128, title: "顔 <&>\"' 🍣",
      palette: { head: "#abcdef", eye: "#123456", bg: "#fefefe" },
    }, true));
  }
}
const encodeNumber = (v: number) => Number.isNaN(v) ? "NaN" : v === Infinity ? "Infinity" : v === -Infinity ? "-Infinity" : v;
const traitCases: any[] = [];
for (const key of BLOB_KEYS) {
  for (const input of [NaN, Infinity, -Infinity, -1, 0, 1, [], [NaN, Infinity, -Infinity], [0.1, 0.6, 0.99]]) {
    const t = traits("edge-seed", true, { [key]: input });
    traitCases.push({ key, input: Array.isArray(input) ? input.map(encodeNumber) : encodeNumber(input), value: t(key) });
  }
}
const output = {
  meta: { schemaVersion: 1, version: "2.7.0", generation: 2, commit, caseCount: cases.length, keys: BLOB_KEYS },
  hash: seeds.flatMap(seed => [true, false].map(normalize => {
    const state = seedState(seed, normalize);
    return { seed, normalize, normalized: normalizeSeed(seed), state: state >>> 0,
      streams: Object.fromEntries(BLOB_KEYS.map((key: string) => [key, stream(state, key)])) };
  })),
  traitCases,
  palette: legacy.palette.map((p: any) => ({ hue: p.hue, tone: p.tone,
    ramp: ramp(p.hue, true, p.tone), rampUnenforced: ramp(p.hue, false, p.tone), hex: palette(p.hue, true, p.tone) })),
  cases,
};
const target = new URL("../crates/blobatar-core/tests/fixtures/gen2-2.7.0.json", import.meta.url);
const bytes = JSON.stringify(output) + "\n";
if (process.argv.includes("--write")) await Bun.write(target, bytes);
else if (await Bun.file(target).text() !== bytes) throw new Error("Fixture differs; inspect before regenerating with --write");
console.log(`${output.cases.length} avatar cases, ${output.hash.length} hash cases, ${traitCases.length} trait cases, ${output.palette.length} palette cases`);
