import { resolve } from "node:path";
import { pathToFileURL } from "node:url";

const reference = process.argv[2];
if (!reference) throw new Error("Usage: bun tools/generate-wall-reference.ts UPSTREAM_CHECKOUT [--write]");
const root = resolve(reference);
const commit = "a7fd546ebede49d0a9fa638945b9e534489782a2";
if ((await Bun.$`git -C ${root} rev-parse HEAD`.quiet()).text().trim() !== commit ||
    (await Bun.$`git -C ${root} status --porcelain --untracked-files=no`.quiet()).text().trim()) {
  throw new Error("Expected a clean pinned upstream checkout");
}
const load = (path: string) => import(pathToFileURL(`${root}/apps/site/${path}.ts`).href);
const geometry = await load("src/wall/geometry");
const camera = await load("src/wall/camera");
const moderation = await load("worker/wall/moderation");
const chunk = await load("src/wall/chunk");
const positions = [-1000000, -65, -33, -32, -31, -1, 0, 1, 31, 32, 33, 65, 1000000];
const cells = positions.flatMap(x => positions.map(y => {
  const at = geometry.chunkOf(x, y);
  const index = geometry.cellIndex(x, y);
  return { x, y, chunk: at, index, restored: geometry.cellAt(at, index) };
}));
const keys = ["0_0", "-0_0", "01_-002", "-9999999_9999999", "10000000_0", "1_2_3", "1.0_2", "+1_2", " 1_2", "1_2\n", "１_２", "-1_-1", "", "a_b"]
  .map(key => ({ key, parsed: geometry.parseChunkKey(key) }));
const boards = [[], [{ x: 0, y: 0 }], [{ x: -32, y: 32 }, { x: -31, y: 32 }, { x: 31, y: -32 }]];
const aims = [[0, 0], [1, 0], [32, 0], [33, 0], [22, 22], [23, 23], [-33, -1], [0, 40], [0, 128], [0, 129], [300, 300]];
const reach = boards.flatMap(occupied => aims.map(([x, y]) => {
  const lookup = (px: number, py: number) => occupied.some(c => c.x === px && c.y === py);
  return { occupied, x, y, populated: occupied.length > 0,
    placeable: geometry.isPlaceable(x, y, lookup, occupied.length > 0),
    nearest: geometry.nearestPlaceable({ x, y }, lookup, occupied.length > 0, 128) };
}));
const names = [null, 12, "", "   ", " Alex ", "\ufeffAlex\ufeff", "\u0085Alex\u0085", "José", "Jose\u0301", "ひろと", "नमस्ते", "李明", "𐐀".repeat(24), "𐐀".repeat(25), "🦀", "a".repeat(24), "a".repeat(25), "O'Neil", "Anne-Marie", "a..b", "a...b", "a  b", "a b", "a_1", ".Alex", "Alex-", "a\u200db", "a\u202eb", "<script>", "n1gg3r", "n i g g e r", "Safe Name", "Café"]
  .map(raw => ({ raw, result: moderation.checkName(raw, "cafe") }));
const expressions = [null, "", "a", "ab", "idle", "thinking", "futurepose", "abcdefghijklmnop", "abcdefghijklmnopq", "Happy", "éé", "aa\n"]
  .map(raw => ({ raw, valid: moderation.checkExpression(raw) }));
const viewport = { width: 800, height: 600 };
const cameras = [{ x: 0, y: 0, zoom: 1 }, { x: -32.25, y: 100, zoom: 0.1 }, { x: 0.5, y: -0.5, zoom: 2 }];
const views = cameras.flatMap(state => [[360, 260], [359, 259], [0, 0], [800, 600], [101, 129]].map(([sx, sy]) => ({
  camera: state, view: viewport, sx, sy,
  cell: camera.cellUnder(state, viewport, sx, sy),
  zoomed: camera.zoomAt(state, viewport, 1.4, sx, sy),
  box: camera.visibleBox(state, viewport, 4),
})));
const bodies = [{ key: "0_0", version: 0, cells: [] },
  { key: "-1_-1", version: 2, cells: [{ index: 1023, seed: "ひろと", expression: "thinking", at: 1791072000 }, { index: 0, seed: "José", expression: "futurepose", at: 1791072001 }] }]
  .map(body => ({ body, encoded: chunk.encodeChunk(body) }));
const value = JSON.stringify({ meta: { commit }, cells, keys, reach, names, expressions, views, bodies }) + "\n";
const output = new URL("../crates/blobatar-wall/tests/fixtures/reference.json", import.meta.url);
if (process.argv.includes("--write")) {
  await Bun.write(output, value);
} else if (await Bun.file(output).text() !== value) {
  throw new Error("Wall reference fixture differs");
}
console.log(`Verified ${cells.length} coordinate, ${keys.length} key, ${reach.length} reach, ${names.length} name, ${expressions.length} expression, ${views.length} camera and ${bodies.length} wire cases`);
