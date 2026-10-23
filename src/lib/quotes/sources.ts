// Where each quote of the day comes from, in the three languages.
//
// A quote names its source with a key:
//   'g:<writing>'  a writing of Nichiren Daishonin (GOSHO below)
//   'nd'           Nichiren Daishonin, writing not given
//   'ls:<n>'       the Lotus Sutra, chapter n
//   'ott'          the Record of the Orally Transmitted Teachings
//   'a:<author>'   another author (AUTHORS below)
//   's:<sutra>'    another sutra (SUTRAS below)
// No key: the quote is shown without a source.
import type { Lang } from '../i18n';

/** [Italian, English, Japanese, source key, pinned day "MM-DD"] */
export type Q = readonly [string, string, string, string?, string?];

type Names = readonly [string, string, string];

const ND: Names = ['Nichiren Daishonin', 'Nichiren Daishonin', '日蓮大聖人'];

const GOSHO: Record<string, Names> = {
  akimoto: ['Lettera ad Akimoto', 'Letter to Akimoto', '秋元御書'],
  attaining: ['Il raggiungimento della Buddità in questa esistenza', 'On Attaining Buddhahood in This Lifetime', '一生成仏抄'],
  believer: ['Risposta a un credente', 'Reply to a Believer', '檀越某御返事'],
  brothers: ['Lettera ai fratelli', 'Letter to the Brothers', '兄弟抄'],
  buddhaland: ['Aspirare alla terra del Budda', 'Aspiration for the Buddha Land', '富木殿御返事'],
  daimoku: ['Il Daimoku del Sutra del Loto', 'The Daimoku of the Lotus Sutra', '法華題目抄'],
  debts: ['Ripagare i debiti di gratitudine', 'On Repaying Debts of Gratitude', '報恩抄'],
  dragon: ['La Porta del Drago', 'The Dragon Gate', '竜門御書'],
  drum: ['Il tamburo alla Porta del Tuono', 'The Drum at the Gate of Thunder', '千日尼御前御返事'],
  evil: ['Grande male e grande bene', 'Great Evil and Great Good', '大悪大善御書'],
  eyes: ['L’apertura degli occhi', 'The Opening of the Eyes', '開目抄'],
  faith: ['Il significato della fede', 'The Meaning of Faith', '妙一尼御前御返事'],
  flowering: ['Fiorire e dare frutto', 'Flowering and Bearing Grain', '華果成就御書'],
  food: ['Le tre virtù del cibo', 'On the Three Virtues of Food', '食物三徳御書'],
  fourteen: ['Le quattordici offese', 'The Fourteen Slanders', '松野殿御返事'],
  gohonzon: ['Il vero aspetto del Gohonzon', 'The Real Aspect of the Gohonzon', '日女御前御返事'],
  happiness: ['La felicità in questo mondo', 'Happiness in This World', '四条金吾殿御返事'],
  heritage: ['L’eredità della Legge suprema della vita', 'The Heritage of the Ultimate Law of Life', '生死一大事血脈抄'],
  hero: ['L’eroe del mondo', 'The Hero of the World', '世雄御書'],
  itai: ['Molti in corpo, uno in mente', 'Many in Body, One in Mind', '異体同心事'],
  kalpa: ['Il kalpa del declino', 'The Kalpa of Decrease', '減劫御書'],
  karma: ['Alleggerire la retribuzione karmica', 'Lessening One’s Karmic Retribution', '転重軽受法門'],
  kyoo: ['Risposta a Kyo’o', 'Reply to Kyo’o', '経王殿御返事'],
  lifespan: ['Prolungare la durata della vita', 'On Prolonging One’s Life Span', '可延定業書'],
  newyear: ['Gosho di Capodanno', 'New Year’s Gosho', '十字御書'],
  nichigon: ['Risposta alla monaca laica Nichigon', 'Reply to the Lay Nun Nichigon', '日厳尼御前御返事'],
  nichiro: ['Lettera al sacerdote Nichiro in prigione', 'Letter to Priest Nichiro in Prison', '土籠御書'],
  niike: ['Lettera a Niike', 'Letter to Niike', '新池御書'],
  object: ['L’oggetto di culto per l’osservazione della mente', 'The Object of Devotion for Observing the Mind', '観心本尊抄'],
  obstacles: ['I tre ostacoli e i quattro demoni', 'The Three Obstacles and Four Devils', '兵衛志殿御返事'],
  persecutions: ['Le persecuzioni che colpiscono il saggio', 'On Persecutions Befalling the Sage', '聖人御難事'],
  personlaw: ['La persona e la Legge', 'The Person and the Law', '南条殿御返事'],
  pondered: ['Il problema su cui riflettere notte e giorno', 'The Problem to Be Pondered Night and Day', '富木殿御書'],
  practicing: ['Praticare gli insegnamenti del Budda', 'On Practicing the Buddha’s Teachings', '如説修行抄'],
  prayer: ['Sulla preghiera', 'On Prayer', '祈祷抄'],
  prophecy: ['Sulla profezia del Budda', 'On the Buddha’s Prophecy', '顕仏未来記'],
  qa: ['Domande e risposte sull’abbracciare il Sutra del Loto', 'Questions and Answers about Embracing the Lotus Sutra', '持妙法華問答抄'],
  rain: ['Tre maestri del Tripitaka pregano per la pioggia', 'Three Tripitaka Masters Pray for Rain', '三三蔵祈雨事'],
  realaspect: ['Il vero aspetto di tutti i fenomeni', 'The True Aspect of All Phenomena', '諸法実相抄'],
  rice: ['Il dono del riso', 'The Gift of Rice', '白米一俵御書'],
  rissho: ['Adottare l’insegnamento corretto per la pace nel paese', 'On Establishing the Correct Teaching for the Peace of the Land', '立正安国論'],
  sado: ['Lettera da Sado', 'Letter from Sado', '佐渡御書'],
  ship: ['Una nave per attraversare il mare della sofferenza', 'A Ship to Cross the Sea of Suffering', '椎地四郎殿御書'],
  stonetiger: ['Il generale Pietra-Tigre', 'General Stone Tiger', '石虎将軍御書'],
  strategy: ['La strategia del Sutra del Loto', 'The Strategy of the Lotus Sutra', '法華経兵法事'],
  supremacy: ['La supremazia della Legge', 'The Supremacy of the Law', '乙御前御消息'],
  sustain: ['La difficoltà di mantenere la fede', 'The Difficulty of Sustaining Faith', '此経難持御書'],
  time: ['La scelta del tempo', 'The Selection of the Time', '撰時抄'],
  tower: ['Sulla torre preziosa', 'On the Treasure Tower', '阿仏房御書'],
  tranquil: ['L’inferno è la Terra della Luce Tranquilla', 'Hell Is the Land of Tranquil Light', '上野殿後家尼御返事'],
  treasures: ['I tre tipi di tesoro', 'The Three Kinds of Treasure', '崇峻天皇御書'],
  twofaith: ['I due tipi di fede', 'The Two Kinds of Faith', '上野殿御返事'],
  unseen: ['Virtù nascosta e ricompensa visibile', 'Unseen Virtue and Visible Reward', '陰徳陽報御書'],
  votary: ['Le azioni del devoto del Sutra del Loto', 'The Actions of the Votary of the Lotus Sutra', '種種御振舞御書'],
  winds: ['Gli otto venti', 'The Eight Winds', '八風抄'],
  winter: ['L’inverno si trasforma sempre in primavera', 'Winter Always Turns to Spring', '妙一尼御前御消息'],
};

/** Chapters of the Lotus Sutra, as Japanese readers know them. */
const LOTUS_JA = [
  '序品第一', '方便品第二', '譬喩品第三', '信解品第四', '薬草喩品第五', '授記品第六', '化城喩品第七',
  '五百弟子受記品第八', '授学無学人記品第九', '法師品第十', '見宝塔品第十一', '提婆達多品第十二',
  '勧持品第十三', '安楽行品第十四', '従地涌出品第十五', '如来寿量品第十六', '分別功徳品第十七',
  '随喜功徳品第十八', '法師功徳品第十九', '常不軽菩薩品第二十', '如来神力品第二十一', '嘱累品第二十二',
  '薬王菩薩本事品第二十三', '妙音菩薩品第二十四', '観世音菩薩普門品第二十五', '陀羅尼品第二十六',
  '妙荘厳王本事品第二十七', '普賢菩薩勧発品第二十八',
];

const AUTHORS: Record<string, Names> = {
  ikeda: ['Daisaku Ikeda', 'Daisaku Ikeda', '池田大作'],
  toda: ['Josei Toda', 'Josei Toda', '戸田城聖'],
  makiguchi: ['Tsunesaburo Makiguchi', 'Tsunesaburo Makiguchi', '牧口常三郎'],
  tiantai: ['T’ien-t’ai', 'T’ien-t’ai', '天台大師'],
  miaolo: ['Miao-lo', 'Miao-lo', '妙楽大師'],
  dengyo: ['Dengyo', 'Dengyo', '伝教大師'],
  nikko: ['Nikko Shonin', 'Nikko Shonin', '日興上人'],
};

const SUTRAS: Record<string, Names> = {
  nirvana: ['Sutra del Nirvana', 'Nirvana Sutra', '涅槃経'],
  meanings: ['Sutra degli innumerevoli significati', 'Immeasurable Meanings Sutra', '無量義経'],
  worthy: ['Sutra della meditazione sul Bodhisattva Virtù Universale', 'Universal Worthy Sutra', '観普賢菩薩行法経'],
  mindground: ['Sutra della contemplazione del terreno della mente', 'Contemplation on the Mind-Ground Sutra', '心地観経'],
};

const OTT: Names = [
  'Nichiren Daishonin, Raccolta degli insegnamenti tramandati oralmente',
  'Nichiren Daishonin, The Record of the Orally Transmitted Teachings',
  '日蓮大聖人『御義口伝』',
];

const IDX: Record<Lang, number> = { it: 0, en: 1, ja: 2 };

/** The source of a quote, written out in `lang` ('' when it has none). */
export function sourceLabel(key: string | undefined, lang: Lang): string {
  if (!key) return '';
  const i = IDX[lang];
  const [kind, name] = key.split(':');
  if (kind === 'nd') return ND[i];
  if (kind === 'ott') return OTT[i];
  if (kind === 'g' && GOSHO[name]) {
    const title = GOSHO[name][i];
    return lang === 'ja' ? `${ND[2]}『${title}』` : `${ND[i]}, ${title}`;
  }
  if (kind === 'ls') {
    const n = Number(name);
    if (lang === 'ja') return `法華経 ${LOTUS_JA[n - 1] ?? ''}`.trim();
    return lang === 'it' ? `Sutra del Loto, cap. ${n}` : `Lotus Sutra, chapter ${n}`;
  }
  if (kind === 'a' && AUTHORS[name]) return AUTHORS[name][i];
  if (kind === 's' && SUTRAS[name]) return SUTRAS[name][i];
  return '';
}

/** Every key used by the quotes must be known: checked by the quote check. */
export function knownSource(key: string | undefined): boolean {
  if (!key) return true;
  const [kind, name] = key.split(':');
  if (kind === 'nd' || kind === 'ott') return name === undefined;
  if (kind === 'g') return name in GOSHO;
  if (kind === 'ls') return Number(name) >= 1 && Number(name) <= 28;
  if (kind === 'a') return name in AUTHORS;
  if (kind === 's') return name in SUTRAS;
  return false;
}
