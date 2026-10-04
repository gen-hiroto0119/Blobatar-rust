import { resolve } from "node:path";
import { mkdir } from "node:fs/promises";
import { pathToFileURL } from "node:url";

const commit = "a7fd546ebede49d0a9fa638945b9e534489782a2";
if (!process.argv[2] || !process.argv[3]) throw new Error("Pass the pinned upstream checkout and output directory");
const root = resolve(process.argv[2]);
const output = resolve(process.argv[3]);
if ((await Bun.$`git -C ${root} rev-parse HEAD`.quiet()).text().trim() !== commit) throw new Error("Wrong upstream revision");
if ((await Bun.$`git -C ${root} status --porcelain --untracked-files=no`.quiet()).text().trim()) throw new Error("Upstream checkout has tracked modifications");
const {blobatar, _layout} = await import(pathToFileURL(`${root}/packages/blobatar/src/blobatar.ts`).href);
await mkdir(output, {recursive:true});
const cases = [];
for (const size of [24,40,64,128,256]) for (const background of ["none","circle","squircle","square"]) for (const surface of ["light","dark"]) {
  const id = `${size}-${background}-${surface}`;
  const avatars = [0.1,0.35,0.55,0.65,0.75,0.82,0.89,0.93,0.965,0.99].map(shape => {
    const options = {size, background:background === "none" ? false : background, traits:{shape}};
    return {seed:"ひろと", options, shape:_layout("ひろと", options).shape, svg:blobatar("ひろと", options)};
  });
  cases.push({id, size, background, surface, avatars});
  const cards = avatars.map(avatar=>`<article><div>${avatar.svg}</div><span>${avatar.shape}</span></article>`).join("");
  await Bun.write(`${output}/${id}.html`, `<!doctype html><html lang="ja"><meta charset="UTF-8"><title>Blobatar ${id}</title><style>
*{box-sizing:border-box}body{margin:0;padding:32px;width:1500px;height:850px;font:16px/24px system-ui,sans-serif;color:${surface === "light" ? "#111318" : "#e7eaf0"};background:${surface === "light" ? "#f4f5f7" : "#111318"}}
h1{margin:0 0 24px;font-size:24px;line-height:36px;font-weight:400}p{margin:0 0 24px}.grid{display:flex;flex-wrap:wrap;gap:16px}article{width:272px;height:312px;text-align:center}article>div{width:272px;height:272px;display:flex;align-items:center;justify-content:center}article span{font-size:14px}svg{display:block;flex:none}
</style><h1>Blobatar / Pinned SVG rendering matrix</h1><p>Seed: ひろと · ${size}px · ${background} · ${surface} surface</p><div class="grid">${cards}</div></html>`);
}
await Bun.write(`${output}/manifest.json`, JSON.stringify({commit,cases},null,2)+"\n");
console.log(`${cases.length} pages, ${cases.reduce((n,c)=>n+c.avatars.length,0)} frozen avatars`);
