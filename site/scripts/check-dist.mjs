import { readFile, readdir } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const dist = fileURLToPath(new URL('../dist/', import.meta.url));
const base = '/chess-chaos-lab/';
const required = ['index.html', 'docs/index.html', 'docs/roadmap/index.html', 'docs/user-guide/index.html', 'docs/architecture/index.html', 'favicon.svg'];
const files = new Set();
async function walk(dir, prefix = '') {
  for (const item of await readdir(dir, { withFileTypes: true })) {
    const name = `${prefix}${item.name}`;
    if (item.isDirectory()) await walk(join(dir, item.name), `${name}/`);
    else files.add(name);
  }
}
await walk(dist);
for (const file of required) if (!files.has(file)) throw new Error(`Missing generated file: ${file}`);
for (const file of files) {
  if (!file.endsWith('.html')) continue;
  const html = await readFile(join(dist, file), 'utf8');
  for (const [, raw] of html.matchAll(/(?:href|src)="([^"]+)"/g)) {
    if (/^(?:https?:|mailto:|data:|#)/.test(raw)) continue;
    const url = new URL(raw, `https://example.test${base}${file}`);
    if (!url.pathname.startsWith(base)) throw new Error(`Link escapes site base: ${file} -> ${raw}`);
    const path = decodeURIComponent(url.pathname.slice(base.length));
    const target = !path || path.endsWith('/') ? `${path}index.html` : path;
    if (!files.has(target)) throw new Error(`Broken local link: ${file} -> ${raw}`);
  }
}
console.log(`Checked ${files.size} generated files and local links`);
