// The text of Gongyo in the international pronunciation, and the timing of
// the Gongyo page's cursor.
//
// The Hoben-pon ends with the ten factors ("Sho-i shoho. Nyo ze so...."),
// recited three times.

export type GongyoPart = 'hoben' | 'juryo';

export interface GongyoLine {
  text: string;
  part: GongyoPart;
  /** 1..3 inside the ten factors of the Hoben-pon. */
  repeat?: number;
}

const HOBEN = `Myo ho ren ge kyo.
Hoben-pon. Dai ni.
Niji seson. Ju sanmai.
Anjo ni ki. Go shari-hotsu.
Sho-but chi-e. Jinjin muryo.
Go chi-e mon. Nange nannyu.
Issai shomon. Hyaku-shi-butsu.
Sho fu no chi. Sho-i sha ga.
Butsu zo shingon. Hyaku sen man noku.
Mushu sho butsu. Jin gyo sho-butsu.
Muryo doho. Yumyo shojin.
Myosho fu mon. Joju jinjin.
Mi-zo-u ho. Zui gi sho setsu.
Ishu nange. Shari-hotsu.
Go ju jo-butsu irai. Shuju innen.
Shuju hiyu. Ko en gonkyo.
Mu shu hoben. Indo shujo.
Ryo ri sho jaku. Sho-i sha ga.
Nyorai hoben. Chi-ken hara-mitsu.
Kai i gu-soku. Shari-hotsu.
Nyorai chi-ken. Kodai jinnon.
Muryo muge. Riki. Mu-sho-i. Zenjo.
Gedas. Sanmai. Jin nyu musai.
Joju issai. Mi-zo-u ho.
Shari-hotsu. Nyorai no.
Shuju fun-betsu. Gyo ses sho ho.
Gon-ji nyunan. Ekka shushin.
Shari-hotsu. Shu yo gon shi.
Muryo muhen. Mi-zo-u ho.
Bus shitsu joju. Shi shari-hotsu.
Fu shu bu setsu. Sho-i sha ga.
Bus sho joju. Dai ichi ke-u.
Nange shi ho. Yui butsu yo butsu.
Nai no kujin. Shoho jisso.`;

const TEN_FACTORS = `Sho-i shoho. Nyo ze so.
Nyo ze sho. Nyo ze tai.
Nyo ze riki. Nyo ze sa.
Nyo ze in. Nyo ze en.
Nyo ze ka. Nyo ze ho.
Nyo ze honmak kukyo to.`;

const JURYO = `Myo ho ren ge kyo.
Nyorai ju-ryo-hon. Dai ju-roku.
Ji ga toku bu rai. Sho kyo sho kosshu.
Muryo hyaku sen man. Oku sai asogi.
Jo seppo kyoke. Mushu oku shujo.
Ryo nyu o butsu-do. Nirai muryo ko.
I do shujo ko. Hoben gen nehan.
Ni jitsu fu metsu-do. Jo ju shi seppo.
Ga jo ju o shi. I sho jin-zu-riki.
Ryo tendo shujo. Sui gon ni fu ken.
Shu ken ga metsu-do. Ko kuyo shari.
Gen kai e renbo. Ni sho katsu-go shin.
Shujo ki shin-bukui. Shichi-jiki i nyunan.
Isshin yok ken butsu. Fu ji shaku shinmyo.
Ji go gyo shuso. Ku shutsu ryojusen.
Ga ji go shujo. Jo zai shi fu-metsu.
I ho-ben-rik ko. Gen u metsu fu-metsu.
Yo-koku u shujo. Kugyo shingyo sha.
Ga bu o hi chu. I setsu mujo ho.
Nyoto fu mon shi. Tan ni ga metsu-do.
Ga ken sho shujo. Motsu-zai o kukai.
Ko fu i gen shin. Ryo go sho katsu-go.
In go shin renbo. Nai shutsu i seppo.
Jin-zu-riki nyo ze. O asogi ko.
Jo zai ryojusen. Gyu yo sho jusho.
Shujo ken ko jin. Dai ka sho sho ji.
Ga shi do annon. Tennin jo juman.
Onrin sho do-kaku. Shuju ho shogon.
Hoju ta keka. Shujo sho yu-raku.
Shoten gyaku tenku. Jo sas shu gi-gaku.
U mandara ke. San butsu gyu daishu.
Ga jodo fu ki. Ni shu ken sho jin.
Ufu sho kuno. Nyo ze shitsu juman.
Ze sho zai shujo. I aku-go innen.
Ka asogi ko. Fu mon sanbo myo.
Sho u shu ku-doku. Nyuwa shichi-jiki sha.
Sokkai ken gashin. Zai shi ni seppo.
Waku-ji-i shi shu. Setsu butsu-ju muryo.
Ku nai ken bussha. I setsu butsu nan chi.
Ga chi-riki nyo ze. Eko sho muryo.
Jumyo mushu ko. Ku shugo sho toku.
Nyoto u chi sha. Mot to shi sho gi.
To dan ryo yo jin. Butsu-go jip puko.
Nyo i zen hoben. I ji o shi ko.
Jitsu zai ni gon shi. Mu no sek komo.
Ga yaku i se bu. Ku sho kugen sha.
I bonbu tendo. Jitsu zai ni gon metsu.
I joken go ko. Ni sho kyoshi shin.
Ho-itsu jaku go-yoku. Da o aku-do chu.
Ga jo chi shujo. Gyo do fu gyo do.
Zui o sho ka do. I ses shuju ho.
Mai ji sa ze nen. I ga ryo shujo.
Toku nyu mu-jo do. Soku joju busshin.`;

const lines = (s: string) => s.split('\n').map((l) => l.trim()).filter(Boolean);

export const GONGYO: GongyoLine[] = [
  ...lines(HOBEN).map((text) => ({ text, part: 'hoben' as const })),
  ...[1, 2, 3].flatMap((repeat) => lines(TEN_FACTORS).map((text) => ({ text, part: 'hoben' as const, repeat }))),
  ...lines(JURYO).map((text) => ({ text, part: 'juryo' as const })),
];

/** Index of the first line of each part. */
export const PART_START: Record<GongyoPart, number> = {
  hoben: 0,
  juryo: GONGYO.findIndex((l) => l.part === 'juryo'),
};

/**
 * How long each character of a line takes, in syllables: a vowel is a
 * syllable, an "n" closing a syllable half of one, a full stop a short
 * breath; the rest (consonants, spaces, hyphens) takes no time of its own.
 */
export function weights(text: string): number[] {
  const t = text.toLowerCase();
  const vowel = (c: string | undefined) => !!c && 'aeiou'.includes(c);
  return [...t].map((c, i) => {
    if (vowel(c)) return vowel(t[i - 1]) && c === 'u' && t[i - 1] === 'o' ? 0.5 : 1;
    if (c === 'n' && !vowel(t[i + 1]) && t[i + 1] !== 'y') return 0.5;
    if (c === '.') return 0.8;
    return 0;
  });
}

/** Seconds a line takes at `speed` syllables a second. */
export function lineSeconds(text: string, speed: number): number {
  return weights(text).reduce((a, w) => a + w, 0) / speed;
}

/**
 * How much of a line has been chanted after `elapsed` seconds: the number
 * of characters, with a fraction for the one under way.
 */
export function cursorAt(text: string, speed: number, elapsed: number): number {
  const w = weights(text);
  let left = elapsed * speed;
  for (let i = 0; i < w.length; i++) {
    if (w[i] === 0) continue;
    if (left < w[i]) return i + left / w[i];
    left -= w[i];
  }
  return text.length;
}
