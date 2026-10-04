import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { createHash } from "node:crypto";

const [reference, archiveRoot] = process.argv.slice(2);
if (!reference || !archiveRoot) {
  throw new Error("Usage: bun tools/generate-generation1-reference.ts UPSTREAM_CHECKOUT V1_ARCHIVE_ROOT [--write]");
}
const root = resolve(reference);
const old = resolve(archiveRoot);
const commit = "a7fd546ebede49d0a9fa638945b9e534489782a2";
const integrity = "sha512-Xl122ZzoiW18Z5sLDHwcZXtHGCQwB1hN9LyUYEdjqBLJltk8h0Ap//1RWq612eABNLE957tii3aBP7hzeRXdsQ==";
if ((await Bun.$`git -C ${root} rev-parse HEAD`.quiet()).text().trim() !== commit ||
    (await Bun.$`git -C ${root} status --porcelain --untracked-files=no`.quiet()).text().trim()) {
  throw new Error("Expected a clean pinned Generation 2 checkout");
}
const archive = `${old}/blobatar-1.0.0.tgz`;
const digest = createHash("sha512").update(new Uint8Array(await Bun.file(archive).arrayBuffer())).digest("base64");
if (`sha512-${digest}` !== integrity) throw new Error("Generation 1 archive integrity mismatch");
const members = (await Bun.$`tar -tzf ${archive}`.quiet()).text().trim().split("\n");
for (const member of members.filter(path => path.startsWith("package/src/") && path.endsWith(".ts"))) {
  const expected = (await Bun.$`tar -xOf ${archive} ${member}`.quiet()).text();
  if (await Bun.file(`${old}/${member}`).text() !== expected) throw new Error(`Modified reference: ${member}`);
}
const load = (path: string) => import(pathToFileURL(path).href);
const { blobatar, layout } = await load(`${old}/package/src/blob.ts`);
const { blobatarUri } = await load(`${old}/package/src/uri.ts`);
const { resolve: resolveOptions } = await load(`${old}/package/src/render.ts`);
const { superellipse, blobPath } = await load(`${old}/package/src/shape.ts`);
// The frozen HTTP handler passes Generation 2 expression values to both renderers.
const expressions = await load(`${root}/packages/blobatar/src/expression.ts`);
const { BLOB_KEYS } = await load(`${root}/packages/blobatar/test/keys.ts`);
const cases: any[] = [];
function snapshot(seed: string, options: any) {
  const opts = { ...options, expression: options.expression ? expressions[options.expression] : undefined };
  const resolved = resolveOptions(seed, opts);
  let l = layout(resolved.t);
  if (opts.expression) l = opts.expression.bake(l, opts.expression.p).l;
  const palette = opts.expression?.tint ? opts.expression.tint(resolved.palette, opts.expression.p) : resolved.palette;
  const b = l.body;
  cases.push({ seed, options, shape: l.shape, body: b, petals: l.petals, eyes: l.eyes, palette,
    bodyPath: ["organic", "cloud"].includes(l.shape) ? blobPath(b.cx, b.cy, b.rx, b.ry, b.radii, b.rot) : superellipse(b),
    eyePaths: l.eyes.map(superellipse), svg: blobatar(seed, opts), uri: blobatarUri(seed, opts) });
}
for (let i = 0; i < 512; i++) snapshot(`generation1-${i}`, {});
for (const seed of ["", "ひろと", "日🦀本", "e\u0301", "é", "ΟΣ", "ΟΣΑ", "İ", "\ufeff ALICE \ufeff", "<>&\"'"]) {
  for (const normalize of [true, false]) snapshot(seed, { normalize });
}
for (const shape of [0.14, 0.43, 0.65, 0.78, 0.885, 0.965]) {
  for (const key of BLOB_KEYS) {
    for (const value of [0, 1]) snapshot(`axis-${key}`, { traits: { shape, [key]: value } });
  }
  for (const expression of ["idle", "happy", "sad", "mad", "surprised", "wink", "sleepy", "smug", "unsure", "scared", "love", "shy", "sick", "thinking"]) {
    for (const background of [false, true, "circle", "square", "squircle"]) {
      snapshot("顔 <&> 🦀", { traits: { shape }, expression, background, size: 128, title: "顔 <&>\"' 🦀", hue: 42, tone: 0.98 });
    }
  }
}
for (const edge of [0.28, 0.58, 0.72, 0.84, 0.93]) {
  for (const offset of [-1e-12, 0, 1e-12]) snapshot("threshold", { traits: { shape: edge + offset } });
}
const output = { meta: { version: "1.0.0", generation: 1, integrity, apiExpressionCommit: commit, caseCount: cases.length }, cases };
const target = new URL("../crates/blobatar-core/tests/fixtures/gen1-1.0.0.json", import.meta.url);
const bytes = JSON.stringify(output) + "\n";
if (process.argv.includes("--write")) await Bun.write(target, bytes);
else if (await Bun.file(target).text() !== bytes) throw new Error("Fixture differs; inspect before regenerating with --write");
console.log(`${cases.length} Generation 1 cases`);
