import { readFile, readdir, mkdir, rm, writeFile, copyFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname, join, basename } from 'node:path';

const site = dirname(dirname(fileURLToPath(import.meta.url)));
const source = join(site, '..', 'docs');
const destination = join(site, 'src', 'content', 'docs', 'docs');
const base = '/chess-chaos-lab/docs';
const files = (await readdir(source)).filter((file) => file.endsWith('.md') && !['final-report.md', 'milestones.md', 'skills-usage.md', 'uci.md'].includes(file));
await rm(join(site, 'src', 'content', 'docs'), { recursive: true, force: true });
await mkdir(destination, { recursive: true });
await mkdir(join(site, 'public', 'screenshots'), { recursive: true });
await copyFile(join(site, '..', 'assets', 'screenshots', 'desktop-windows-x64.jpg'), join(site, 'public', 'screenshots', 'desktop-windows-x64.jpg'));

for (const file of files) {
  const original = await readFile(join(source, file), 'utf8');
  const title = original.match(/^# (.+)$/m)?.[1];
  if (!title) throw new Error(`${file} needs a level-one heading`);
  const linked = original.replace(/^# .+\r?\n/, '').replace(/\]\(([^)#]+\.md)(#[^)]+)?\)/g, (_match, path, hash = '') => {
    if (path.startsWith('../')) return `](https://github.com/ViolaPeracia/chess-chaos-lab/blob/main/${path.slice(3)}${hash})`;
    if (path.includes('/')) throw new Error(`Unexpected documentation link: ${path} in ${file}`);
    const slug = basename(path, '.md').toLowerCase();
    return `](${base}/${slug === 'index' ? '' : `${slug}/`}${hash})`;
  }).replace(/\]\(\.\.\/([^)]+)\)/g, (_match, path) => `](https://github.com/ViolaPeracia/chess-chaos-lab/blob/main/${path})`);
  const name = file.toLowerCase();
  await writeFile(join(destination, name), `---\ntitle: ${JSON.stringify(title)}\n---\n\n${linked}`);
}
console.log(`Synced ${files.length} canonical docs pages`);
