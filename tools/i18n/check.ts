// Checks every language in src/lib/locales against English:
//   • the same texts (keys, quotes, Gongyo lines), none empty;
//   • the same {placeholders} in every text of the screens;
//   • a guide whose lists stay lists ("• " lines).
// TypeScript already checks the shape; this checks what it cannot.
//
//     node --experimental-strip-types tools/i18n/check.ts [code...]
import { readdirSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const LOCALES = join(dirname(fileURLToPath(import.meta.url)), '..', '..', 'src', 'lib', 'locales');
const PARTS = ['ui', 'help', 'dictionary', 'quotes', 'gongyo', 'sources'] as const;

type Tree = string | Tree[] | { [k: string]: Tree };

async function load(code: string): Promise<Record<string, Tree>> {
  const out: Record<string, Tree> = {};
  for (const part of PARTS) out[part] = (await import(join(LOCALES, code, `${part}.ts`))).default;
  out.meta = (await import(join(LOCALES, code, 'meta.ts'))).default;
  return out;
}

/** Every text with its path: "ui.count.today", "quotes.gosho.3"... */
function leaves(t: Tree, path: string, out = new Map<string, string>()): Map<string, string> {
  if (typeof t === 'string') out.set(path, t);
  else if (Array.isArray(t)) t.forEach((x, i) => leaves(x, `${path}.${i}`, out));
  else for (const [k, v] of Object.entries(t)) leaves(v, `${path}.${k}`, out);
  return out;
}

const holes = (s: string) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort().join(',');
const bullets = (s: string) => s.split('\n').filter((l) => l.startsWith('• ')).length;

const en = await load('en');
const ref = new Map<string, string>();
for (const part of PARTS) leaves(en[part], part, ref);

const codes = process.argv.slice(2).length ? process.argv.slice(2) : readdirSync(LOCALES).filter((f) => !f.includes('.') && f !== 'en');
let bad = 0;
for (const code of codes.sort()) {
  const loc = await load(code);
  const mine = new Map<string, string>();
  for (const part of PARTS) leaves(loc[part], part, mine);
  const problems: string[] = [];
  for (const [path, s] of ref) {
    const m = mine.get(path);
    if (m === undefined) {
      if (!path.startsWith('sources.lotusChapters')) problems.push(`missing ${path}`);
      continue;
    }
    if (!m.trim()) problems.push(`empty ${path}`);
    if (path.startsWith('ui.') && holes(m) !== holes(s)) problems.push(`placeholders ${path}: {${holes(m)}} instead of {${holes(s)}}`);
    if (path.startsWith('help.') && bullets(m) !== bullets(s)) problems.push(`list items ${path}: ${bullets(m)} instead of ${bullets(s)}`);
  }
  for (const path of mine.keys()) if (!ref.has(path) && !path.startsWith('sources.lotusChapters')) problems.push(`unknown ${path}`);
  const same = [...mine].filter(([p, s]) => ref.get(p) === s && /[a-z]{4}/.test(s) && !p.startsWith('sources.')).length;
  const meta = loc.meta as { name: string; english: string };
  console.log(`${code} ${meta.name} (${meta.english}): ${problems.length ? `${problems.length} problems` : 'ok'}${same ? `, ${same} texts as in English` : ''}`);
  for (const p of problems.slice(0, 20)) console.log(`   ${p}`);
  bad += problems.length;
}
process.exit(bad ? 1 : 0);
