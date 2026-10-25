// The shape of a language (src/lib/locales/<code>/, see README.md). English
// (en) is the reference: every other language has the same keys, as many
// quotes and as many Gongyo lines, and TypeScript checks it.
import type { TermId } from '../dictionary';
import type { ORIGINAL } from '../gongyoTranslation';
import type { HelpId } from '../help';
import type { POOLS } from '../quotes/pools';
import type { AuthorId, GoshoId, SutraId } from '../quotes/sources';
import type ui from './en/ui';

/** The key of a text of the app's screens. */
export type Key = keyof typeof ui;
export type Dict = Record<Key, string>;

/** Exactly N strings. */
export type Strings<N extends number, A extends string[] = []> = A['length'] extends N ? A : Strings<N, [...A, string]>;
type Like<T extends readonly unknown[]> = Strings<T['length']>;

export interface LocaleMeta {
  /** The language's name in the language itself. */
  name: string;
  /** Its name in English (also searched in the language list). */
  english: string;
  /** The tag for dates and numbers (Intl). */
  intl: string;
  /** Sort the dictionary by the second line of its entries (Japanese: by reading). */
  sortByGloss?: boolean;
}

export type HelpTexts = Record<HelpId, readonly [title: string, body: string]>;

/** Headword, second line (a gloss, or the reading) and explanation. */
export type DictionaryTexts = Record<TermId, readonly [title: string, gloss: string, text: string]>;

export type QuoteTexts = { [P in keyof typeof POOLS]: Like<(typeof POOLS)[P]> };

export type GongyoTexts = { [P in keyof typeof ORIGINAL]: Like<(typeof ORIGINAL)[P]> };

/** Where the quotes come from. */
export interface SourceTexts {
  /** Nichiren Daishonin. */
  nichiren: string;
  /** A writing of his: {author} and {title}. */
  writing: string;
  /** The Record of the Orally Transmitted Teachings, with its author. */
  ott: string;
  /** A chapter of the Lotus Sutra: {n}, or {chapter} from `lotusChapters`. */
  lotus: string;
  lotusChapters?: Strings<28>;
  gosho: Record<GoshoId, string>;
  authors: Record<AuthorId, string>;
  sutras: Record<SutraId, string>;
}

export interface Locale {
  ui: Dict;
  help: HelpTexts;
  dictionary: DictionaryTexts;
  quotes: QuoteTexts;
  gongyo: GongyoTexts;
  sources: SourceTexts;
}
