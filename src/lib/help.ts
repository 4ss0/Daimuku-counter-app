// "How to use" page: one section per topic. The titles and texts, in each
// language, are in src/lib/locales/<code>/help.ts. In a body, a blank line
// starts a new paragraph and a line starting with "• " is a list item.

export type HelpIcon = 'voice' | 'count' | 'coin' | 'moon' | 'tips' | 'stats' | 'goal' | 'badge' | 'bell' | 'backup' | 'menu' | 'help';

export const HELP = [
  { id: 'voice', icon: 'voice' },
  { id: 'count', icon: 'count' },
  { id: 'coins', icon: 'coin' },
  { id: 'screen', icon: 'moon' },
  { id: 'tips', icon: 'tips' },
  { id: 'stats', icon: 'stats' },
  { id: 'goals', icon: 'goal' },
  { id: 'badges', icon: 'badge' },
  { id: 'bell', icon: 'bell' },
  { id: 'backup', icon: 'backup' },
  { id: 'menu', icon: 'menu' },
  { id: 'trouble', icon: 'help' },
] as const satisfies readonly { id: string; icon: HelpIcon }[];

export type HelpId = (typeof HELP)[number]['id'];

/** Paragraphs and lists of a section body. */
export function blocks(body: string): ({ p: string } | { list: string[] })[] {
  return body.split('\n\n').map((chunk) => {
    const lines = chunk.split('\n');
    return lines.every((l) => l.startsWith('• ')) ? { list: lines.map((l) => l.slice(2)) } : { p: chunk };
  });
}
