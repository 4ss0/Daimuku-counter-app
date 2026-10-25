import type { Locale } from '../types';
import dictionary from './dictionary';
import gongyo from './gongyo';
import help from './help';
import quotes from './quotes';
import sources from './sources';
import ui from './ui';

export default { ui, help, dictionary, quotes, gongyo, sources } satisfies Locale;
