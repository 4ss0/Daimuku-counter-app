// The text of Gongyo in the international pronunciation, and its rhythm.
//
// Gongyo is recited one character of the sutra (one kanji) per beat, every
// beat the same length, with no pause at the full stops nor between lines:
// in a recording of slow Gongyo every beat lasts 0.39 s, the Hoben-pon
// (378 beats) 141 s and the Juryo-hon (523 beats) 202 s. So the text is cut
// into the readings of the kanji ("toku", "butsu", "shu", "jo"...) and each
// takes one beat, whatever its length in letters.
//
// The Hoben-pon ends with the ten factors ("Sho-i shoho. Nyo ze so...."),
// recited three times.

export type GongyoPart = 'hoben' | 'juryo';

/** A piece of a line: the reading of one kanji (a beat), or what lies between. */
export interface Piece {
  text: string;
  beat: boolean;
}

export interface GongyoLine {
  text: string;
  part: GongyoPart;
  /** 1..3 inside the ten factors of the Hoben-pon. */
  repeat?: number;
  pieces: Piece[];
  beats: number;
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
Shujo ki shin-buku. Shichi-jiki i nyunan.
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

const VOWELS = 'aeiou';

/**
 * The readings of the kanji in one romanized word: "Shari" -> sha, ri;
 * "toku" -> toku; "jinjin" -> jin, jin; "Issai" -> is, sai; "butsu" -> butsu.
 * A reading is consonants and a vowel (ai, ei, ui count as one), closed by an
 * "n", by the first of a double consonant, by a final k/s/t/p ("but", "das"),
 * or ending in -ku, -tsu, or -ki/-chi after an i ("riki", "ichi").
 */
export function readings(word: string): string[] {
  const t = word.toLowerCase();
  const out: string[] = [];
  let i = 0;
  while (i < t.length) {
    let j = i;
    while (j < t.length && !VOWELS.includes(t[j])) j++;
    if (j >= t.length) {
      if (out.length) out[out.length - 1] += word.slice(i);
      else out.push(word.slice(i));
      break;
    }
    const v = t[j++];
    if ('aeu'.includes(v) && t[j] === 'i' && !VOWELS.includes(t[j + 1] ?? '#')) j++;
    const rest = t.slice(j);
    if (rest === 'tsu' || rest === 'ku' || ((rest === 'ki' || rest === 'chi') && v === 'i')) j = t.length;
    else if (rest[0] === 'n' && !(VOWELS + 'y').includes(rest[1] ?? '#')) j++;
    else if (rest.length >= 2 && rest[0] === rest[1] && !VOWELS.includes(rest[0])) j++;
    else if (rest.length === 1 && 'kstp'.includes(rest)) j++;
    out.push(word.slice(i, j));
    i = j;
  }
  return out;
}

/** A line cut into beats and what lies between them (spaces, hyphens, stops). */
export function pieces(text: string): Piece[] {
  const out: Piece[] = [];
  for (const part of text.split(/([\s.\-]+)/)) {
    if (!part) continue;
    if (/^[\s.\-]+$/.test(part)) out.push({ text: part, beat: false });
    else for (const r of readings(part)) out.push({ text: r, beat: true });
  }
  return out;
}

const lines = (s: string) => s.split('\n').map((l) => l.trim()).filter(Boolean);

function line(text: string, part: GongyoPart, repeat?: number): GongyoLine {
  const p = pieces(text);
  return { text, part, repeat, pieces: p, beats: p.filter((x) => x.beat).length };
}

export const GONGYO: GongyoLine[] = [
  ...lines(HOBEN).map((t) => line(t, 'hoben')),
  ...[1, 2, 3].flatMap((r) => lines(TEN_FACTORS).map((t) => line(t, 'hoben', r))),
  ...lines(JURYO).map((t) => line(t, 'juryo')),
];

/** Index of the first line of each part. */
export const PART_START: Record<GongyoPart, number> = {
  hoben: 0,
  juryo: GONGYO.findIndex((l) => l.part === 'juryo'),
};

/** Beats before each line, and in all. */
export const BEATS_BEFORE: number[] = GONGYO.reduce<number[]>((a, l, i) => (a.push(i ? a[i - 1] + GONGYO[i - 1].beats : 0), a), []);
export const TOTAL_BEATS = BEATS_BEFORE[GONGYO.length - 1] + GONGYO[GONGYO.length - 1].beats;

/** Speeds in beats a second: slow is the recording above. */
export const PACE = { min: 1.8, slow: 2.6, medium: 3.3, fast: 4.2, max: 5.2 };
