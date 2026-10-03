import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { createHash } from "node:crypto";

const [reference, archiveRoot] = process.argv.slice(2);
if (!reference || !archiveRoot) throw new Error("Usage: bun tools/generate-api-reference.ts UPSTREAM_CHECKOUT V1_ARCHIVE_ROOT [--write]");
const root = resolve(reference);
const old = resolve(archiveRoot);
const expectedCommit = "a7fd546ebede49d0a9fa638945b9e534489782a2";
if ((await Bun.$`git -C ${root} rev-parse HEAD`.quiet()).text().trim() !== expectedCommit ||
    (await Bun.$`git -C ${root} status --porcelain --untracked-files=no`.quiet()).text().trim()) {
  throw new Error("Expected a clean pinned upstream checkout");
}
const archive = `${old}/blobatar-1.0.0.tgz`;
const integrity = "sha512-Xl122ZzoiW18Z5sLDHwcZXtHGCQwB1hN9LyUYEdjqBLJltk8h0Ap//1RWq612eABNLE957tii3aBP7hzeRXdsQ==";
const digest = createHash("sha512").update(new Uint8Array(await Bun.file(archive).arrayBuffer())).digest("base64");
if (`sha512-${digest}` !== integrity) throw new Error("Generation 1 archive integrity mismatch");
for (const member of (await Bun.$`tar -tzf ${archive}`.quiet()).text().trim().split("\n").filter(p => p.startsWith("package/src/") && p.endsWith(".ts"))) {
  if (await Bun.file(`${old}/${member}`).text() !== (await Bun.$`tar -xOf ${archive} ${member}`.quiet()).text()) {
    throw new Error(`Modified reference: ${member}`);
  }
}
const sourcePlugin = { name: "frozen-blobatar-sources", setup(build: any) {
  build.onResolve({ filter: /^blobatar(?:-v1)?(?:\/.*)?$/ }, args => {
    const legacy = args.path.startsWith("blobatar-v1");
    const entry = args.path.split("/")[1] ?? "index";
    return { path: `${legacy ? `${old}/package` : `${root}/packages/blobatar`}/src/${entry}.ts` };
  });
}};
const outdir = new URL("../target/api-reference/", import.meta.url).pathname;
const built = await Bun.build({ entrypoints: ["index", "avatar", "openapi", "errors"].map(name => `${root}/apps/api/src/${name}.ts`), outdir, target: "bun", plugins: [sourcePlugin] });
if (!built.success) throw new Error(built.logs.join("\n"));
const load = (file: string) => import(pathToFileURL(`${outdir}${file}.js`).href);
const { default: api } = await load("index");
const { usage } = await load("avatar");
const { openapi } = await load("openapi");
const { ERROR_CODES, errorBody } = await load("errors");
const cases: any[] = [];
const origin = "http://127.0.0.1:3000";
async function request(path: string, method = "GET", headers: Record<string, string> = {}) {
  const req = new Request(`${origin}${path}`, { method, headers });
  const response = api.fetch(req);
  cases.push({ method, url: req.url, headers, status: response.status,
    responseHeaders: Object.fromEntries(response.headers),
    // A real HTTP server strips HEAD bodies, even for upstream's error/help responses.
    body: method === "HEAD" ? "" : await response.text() });
  return response.headers.get("etag");
}
for (const gen of ["", "1", "2"]) {
  for (const expression of ["idle", "happy", "sad", "mad", "surprised", "wink", "sleepy", "smug", "unsure", "scared", "love", "shy", "sick", "thinking"]) {
    await request(`/avatar/${encodeURIComponent("ひろと🦀")}?expression=${expression}${gen ? `&gen=${gen}` : ""}&title=${encodeURIComponent("顔 <&>\"' 🦀")}`);
  }
}
for (const query of ["", "size=64&hue=200&tone=0.25&title=Alain", "s=64&size=256", "s=&size=64", "s=64&s=128", "d=404&default=identicon&f=y&forcedefault=y&r=g&rating=x", ...["none", "circle", "square", "squircle"].map(b => `background=${b}`), ...["", " ", "abc", "NaN", "Infinity", "-Infinity", "2048", "1", "32.6", "0x40", "0b100000", "0o100", "1e2", ".5", "1.", "-0", "0XFF", "\ufeff64\ufeff"].map(s => `size=${encodeURIComponent(s)}`)]) {
  await request(`/avatar/alain00?${query}`);
}
for (const path of ["/", "/avatar/", "/openapi.json", "/wrong", "/avatar/alain%40example.com", "/avatar/a%2Fb", "/avatar/.svg.svg", ...["svg", "png", "jpg", "jpeg", "gif", "webp", "PNG"].map(e => `/avatar/alain.${e}`)]) {
  await request(path);
  await request(path, "HEAD");
}
const errors = ["/wrong", "/avatar/a/b", "/avatar/%", "/avatar/%GG", "/avatar/%FF", "/avatar/%ED%A0%80", "/avatar/.svg", `/avatar/${"a".repeat(257)}`, `/avatar/${encodeURIComponent("🦀".repeat(129))}`,
  ...["expresion=happy", "utm_source=x", "gen=3", "gen=01", "gen=", "expression=poseVars", "expression=constructor", "background=__proto__", "gen=hasOwnProperty", "background=false", "tone=1.1", "hue=361", "hue=-1", "hue=1e100", "tone=abc", "tone=NaN", "hue=", "hue=%20", "hue=%C2%85", "hue=%2B0x40", "hue=0xg", "title=" + "a".repeat(129), "title=" + encodeURIComponent("🦀".repeat(65))].map(q => `/avatar/alain?${q}`)];
for (const path of errors) {
  await request(path);
  await request(path, "GET", { accept: "application/json" });
  await request(path, "HEAD", { accept: "application/problem+json" });
}
for (const method of ["POST", "PUT", "DELETE", "OPTIONS", "PATCH"]) {
  await request("/avatar/alain", method);
  await request("/avatar/alain", method, { accept: "application/json" });
}
for (const accept of ["*/*", "text/html", "application/json;q=0", "application/problem+json", "APPLICATION/JSON"]) {
  await request("/avatar/alain?oops=1", "GET", { accept });
}
for (const query of ["hue=0x40", "hue=0b100", "hue=0o100", "hue=%20%2B12%20", "hue=%EF%BB%BF12%EF%BB%BF", "hue=360&tone=1", "hue=0&tone=0", "title=" + "a".repeat(128), "title=" + encodeURIComponent("🦀".repeat(64))]) await request(`/avatar/alain?${query}`);
await request(`/avatar/${encodeURIComponent("🦀".repeat(128))}`);
const tag = await request("/avatar/etag?gen=1");
for (const value of [tag!, '"stale"', `W/${tag}`, `${tag}, "other"`, "*"]) {
  for (const method of ["GET", "HEAD"]) await request("/avatar/etag?gen=1", method, { "if-none-match": value });
}
const target = "../crates/blobatar-server/";
const outputs = {
  "tests/fixtures/api.json": JSON.stringify({ meta: { commit: expectedCommit, generation1Integrity: integrity, caseCount: cases.length }, cases }) + "\n",
  "src/reference/usage.txt": await usage().text(),
  "src/reference/openapi.json": JSON.stringify(openapi(origin), null, 2) + "\n",
  "src/reference/errors.json": JSON.stringify(Object.fromEntries(ERROR_CODES.map((code: string) => [code, errorBody(400, code, "").error.hint])), null, 2) + "\n",
};
for (const [file, bytes] of Object.entries(outputs)) {
  const path = new URL(target + file, import.meta.url);
  if (process.argv.includes("--write")) await Bun.write(path, bytes);
  else if (await Bun.file(path).text() !== bytes) throw new Error(`Fixture differs: ${file}`);
}
console.log(`${cases.length} HTTP reference cases`);
