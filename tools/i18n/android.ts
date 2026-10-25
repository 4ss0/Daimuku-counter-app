// Writes the Android texts of every language (the names of the notification
// channels) from src/lib/locales/<code>/ui.ts into
// src-tauri/gen/android/app/src/main/res/values-<code>/strings.xml.
//
//     node --experimental-strip-types tools/i18n/android.ts
//
// English goes into values/strings.xml, next to the app's name.
import { mkdirSync, readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const LOCALES = join(ROOT, 'src', 'lib', 'locales');
const RES = join(ROOT, 'src-tauri', 'gen', 'android', 'app', 'src', 'main', 'res');

/** Android's folder for a language, where it differs from the code. */
const FOLDER: Record<string, string> = { id: 'in', no: 'nb', fil: 'b+fil' };

const STRINGS: [name: string, key: string][] = [
  ['channel_counting', 'android.channelCounting'],
  ['channel_counting_text', 'android.channelCountingText'],
  ['channel_reminders', 'android.channelReminders'],
  ['channel_reminders_text', 'android.channelRemindersText'],
];

const xml = (s: string) =>
  s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/'/g, "\\'").replace(/"/g, '\\"');

for (const code of readdirSync(LOCALES).filter((f) => !f.includes('.')).sort()) {
  const ui: Record<string, string> = (await import(join(LOCALES, code, 'ui.ts'))).default;
  const lines = STRINGS.map(([name, key]) => `    <string name="${name}">${xml(ui[key])}</string>`);
  const dir = join(RES, code === 'en' ? 'values' : `values-${FOLDER[code] ?? code}`);
  mkdirSync(dir, { recursive: true });
  if (code === 'en') {
    // keep the app's name, which is not translated
    const old = readFileSync(join(dir, 'strings.xml'), 'utf-8');
    const keep = old.split('\n').filter((l) => /name="(app_name|main_activity_title)"/.test(l));
    lines.unshift(...keep);
  }
  writeFileSync(join(dir, 'strings.xml'), `<resources>\n${lines.join('\n')}\n</resources>\n`);
  console.log(code, '->', dir.slice(RES.length + 1));
}
