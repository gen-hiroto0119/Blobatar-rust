import { build } from 'esbuild';
import { fileURLToPath } from 'node:url';

const directory = fileURLToPath(new URL('.', import.meta.url));
const endpoint = process.env.NEON_PUBLIC_DATABASE_URL || '';
if (endpoint) {
  const url = new URL(endpoint);
  if (url.protocol !== 'https:' || !url.hostname.endsWith('.neon.tech') ||
      url.username || url.password || url.search || url.hash) {
    throw new Error('NEON_PUBLIC_DATABASE_URL must be the public HTTPS Neon URL without credentials or query parameters, never DATABASE_URL.');
  }
}
await build({
  absWorkingDir: directory,
  entryPoints: ['src/cloud.js'],
  outfile: 'assets/cloud-client.js',
  bundle: true,
  minify: true,
  format: 'iife',
  target: ['es2022'],
  define: { NEON_PUBLIC_DATABASE_URL: JSON.stringify(endpoint) },
});
