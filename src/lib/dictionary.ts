// Dictionary of the main concepts of Nichiren Daishonin's Buddhism.
// Every entry: Japanese term and reading; for Italian and English the
// headword, a short gloss and the explanation; for Japanese the
// explanation (the headword is the term itself).
import type { Lang } from './i18n';

export type Cat = 'practice' | 'doctrine' | 'history';

export interface Term {
  id: string;
  cat: Cat;
  jp: string;
  kana: string;
  it: [title: string, gloss: string, text: string];
  en: [title: string, gloss: string, text: string];
  ja: string;
  see?: string[];
}

export const CATS: Cat[] = ['practice', 'doctrine', 'history'];

/** Headword, second line (and the Japanese term next to it) and text of an entry in a language. */
export function view(e: Term, lang: Lang): { title: string; sub: string; jp: string; text: string } {
  if (lang === 'ja') return { title: e.jp, sub: e.kana, jp: '', text: e.ja };
  const [title, sub, text] = e[lang];
  return { title, sub, jp: e.jp, text };
}

/** Everything a search may match, in every language, lower case. */
export function haystack(e: Term): string {
  return [e.jp, e.kana, ...e.it, ...e.en, e.ja].join('\n').toLowerCase();
}

export const TERMS: Term[] = [
  // ---------------------------------------------------------------- practice
  {
    id: 'daimoku',
    cat: 'practice',
    jp: '題目',
    kana: 'だいもく',
    it: [
      'Daimoku',
      'Recitazione di Nam-myoho-renge-kyo',
      'Letteralmente «titolo»: il titolo del Sutra del Loto, Myoho-renge-kyo. Nella pratica indica la recitazione ripetuta di Nam-myoho-renge-kyo davanti al Gohonzon, cuore della pratica insegnata da Nichiren Daishonin. Si recita per far emergere la Buddità nella propria vita, per sé e per gli altri: è ciò che questa app conta.',
    ],
    en: [
      'Daimoku',
      'Chanting Nam-myoho-renge-kyo',
      'Literally "title": the title of the Lotus Sutra, Myoho-renge-kyo. In practice it means chanting Nam-myoho-renge-kyo to the Gohonzon, the core of the practice taught by Nichiren Daishonin. One chants to bring forth Buddhahood in one’s own life, for oneself and for others: it is what this app counts.',
    ],
    ja: '本来は「題名」の意で、法華経の題名である妙法蓮華経を指す。実践としては、御本尊に向かって南無妙法蓮華経と唱えること（唱題）をいい、日蓮大聖人が説いた修行の根本である。自他共に自身の仏界を涌現させるために唱える。このアプリが数えるのがこの題目である。',
    see: ['nmrk', 'gongyo', 'gohonzon', 'sandai-hiho'],
  },
  {
    id: 'nmrk',
    cat: 'practice',
    jp: '南無妙法蓮華経',
    kana: 'なんみょうほうれんげきょう',
    it: [
      'Nam-myoho-renge-kyo',
      'La Legge mistica',
      'La Legge fondamentale della vita e dell’universo secondo Nichiren Daishonin, che la proclamò nel 1253. «Nam» (dal sanscrito namas) significa dedicarsi; «Myoho» è la Legge mistica; «renge», il fiore di loto, che fiorisce e dà semi nello stesso momento, simboleggia la simultaneità di causa ed effetto; «kyo» significa insegnamento, e anche suono e voce.',
    ],
    en: [
      'Nam-myoho-renge-kyo',
      'The Mystic Law',
      'The fundamental Law of life and the universe according to Nichiren Daishonin, who first proclaimed it in 1253. "Nam" (from the Sanskrit namas) means devotion; "Myoho" is the Mystic Law; "renge", the lotus flower, which blooms and seeds at the same time, stands for the simultaneity of cause and effect; "kyo" means teaching, and also sound or voice.',
    ],
    ja: '日蓮大聖人が1253年（建長5年）に初めて唱え出した、宇宙と生命を貫く根本の法。「南無」はサンスクリット語ナマスの音写で帰命（身を捧げること）を意味する。「妙法」は不思議な法、「蓮華」は花と実が同時に具わる蓮で因果倶時を表し、「経」は教え、また声・響きを意味する。',
    see: ['daimoku', 'inga-guji', 'nichiren', 'lotus-sutra'],
  },
  {
    id: 'gongyo',
    cat: 'practice',
    jp: '勤行',
    kana: 'ごんぎょう',
    it: [
      'Gongyo',
      'Pratica quotidiana',
      'Letteralmente «pratica assidua». Mattina e sera si recitano davanti al Gohonzon parti del secondo capitolo (Hoben) e del sedicesimo (Juryo) del Sutra del Loto, seguite dal daimoku. Il daimoku è la pratica principale; la recitazione dei due capitoli ne è il supporto.',
    ],
    en: [
      'Gongyo',
      'Daily practice',
      'Literally "assiduous practice". Morning and evening one recites, before the Gohonzon, portions of the second ("Expedient Means") and sixteenth ("The Life Span of the Thus Come One") chapters of the Lotus Sutra, followed by daimoku. Daimoku is the primary practice; reciting the two chapters is the supporting one.',
    ],
    ja: '「勤めて行ずる」こと。朝夕、御本尊に向かって法華経の方便品第二と如来寿量品第十六を読誦し、題目を唱える。唱題が正行、方便品・寿量品の読誦が助行である。',
    see: ['daimoku', 'gohonzon', 'lotus-sutra'],
  },
  {
    id: 'gohonzon',
    cat: 'practice',
    jp: '御本尊',
    kana: 'ごほんぞん',
    it: [
      'Gohonzon',
      'Oggetto di culto',
      'Il mandala scritto da Nichiren Daishonin, con Nam-myoho-renge-kyo al centro e intorno i rappresentanti dei Dieci mondi. Esprime lo stato di Buddità che esiste in ogni persona: recitando davanti a esso il praticante lo riconosce e lo fa emergere nella propria vita. «Go» è un prefisso di rispetto, «honzon» significa oggetto di fondamentale rispetto.',
    ],
    en: [
      'Gohonzon',
      'Object of devotion',
      'The mandala inscribed by Nichiren Daishonin, with Nam-myoho-renge-kyo down the centre surrounded by representatives of the Ten Worlds. It embodies the state of Buddhahood present in every person: chanting to it, practitioners recognise and bring forth that state in their own lives. "Go" is an honorific, "honzon" means object of fundamental respect.',
    ],
    ja: '日蓮大聖人が図顕した曼荼羅で、中央に南無妙法蓮華経が認められ、その周りに十界の衆生が配されている。万人の生命に具わる仏界を表したもので、これに向かって唱題することで自身の仏界を涌現させる。「本尊」は根本として尊敬するものの意。',
    see: ['jikkai', 'butsudan', 'sandai-hiho', 'kyochi-myogo'],
  },
  {
    id: 'butsudan',
    cat: 'practice',
    jp: '仏壇',
    kana: 'ぶつだん',
    it: [
      'Butsudan',
      'Altare',
      'Il mobiletto, di solito in legno, che custodisce e protegge il Gohonzon in casa. Davanti a esso si dispongono di solito sempreverdi, candele, incenso, acqua e frutta come offerte.',
    ],
    en: [
      'Butsudan',
      'Altar',
      'The cabinet, usually made of wood, that houses and protects the Gohonzon at home. Evergreens, candles, incense, water and fruit are usually set before it as offerings.',
    ],
    ja: '家庭で御本尊を安置し、お守りするための厨子（多くは木製）。前には樒（しきみ）、ろうそく、線香、水、果物などを供える。',
    see: ['gohonzon'],
  },
  {
    id: 'juzu',
    cat: 'practice',
    jp: '数珠',
    kana: 'じゅず',
    it: [
      'Juzu',
      'Rosario buddista',
      'Il rosario che si tiene tra le mani giunte durante gongyo e daimoku. Ha 108 grani principali, che secondo la tradizione rappresentano i desideri terreni; si appoggia sulle dita medie delle due mani, attorcigliato una volta al centro.',
    ],
    en: [
      'Juzu',
      'Prayer beads',
      'The beads held between the joined hands during gongyo and daimoku. They have 108 main beads, traditionally said to stand for earthly desires; they rest on the middle fingers of both hands, twisted once in the middle.',
    ],
    ja: '勤行・唱題の際に合掌した手にかける念珠。主な珠は108個で、煩悩を表すとされる。両手の中指にかけ、中央で一度ひねって用いる。',
    see: ['gongyo', 'bonno-soku-bodai'],
  },
  {
    id: 'shinjin',
    cat: 'practice',
    jp: '信心',
    kana: 'しんじん',
    it: [
      'Fede',
      'Shinjin',
      'Nel Buddismo di Nichiren la fede non è credere ciecamente, ma la fiducia nel potenziale illimitato della propria vita e di quella degli altri, rinnovata ogni giorno. Nichiren la paragona all’acqua che scorre senza fermarsi, contrapposta alla fede che divampa come il fuoco e poi si spegne.',
    ],
    en: [
      'Faith',
      'Shinjin',
      'In Nichiren Buddhism faith is not blind belief but confidence in the unlimited potential of one’s own life and of others’ lives, renewed every day. Nichiren compares it to water that flows without stopping, as opposed to faith that flares up like fire and then dies down.',
    ],
    ja: '日蓮仏法の信心は盲信ではなく、自他の生命に具わる無限の可能性を信じ、日々新たにしていくことである。大聖人は、燃え上がってはすぐ消える火のような信心ではなく、絶えず流れる水のような信心を勧めている。',
    see: ['shin-gyo-gaku', 'shiriki'],
  },
  {
    id: 'shin-gyo-gaku',
    cat: 'practice',
    jp: '信・行・学',
    kana: 'しん・ぎょう・がく',
    it: [
      'Fede, pratica e studio',
      'Shin, gyo, gaku',
      'I tre fondamenti della pratica buddista. La fede è la base; la pratica comprende la pratica per sé (gongyo e daimoku) e per gli altri (diffondere l’insegnamento); lo studio degli scritti di Nichiren approfondisce e rafforza la fede.',
    ],
    en: [
      'Faith, practice and study',
      'Shin, gyo, gaku',
      'The three basics of Buddhist practice. Faith is the foundation; practice includes practice for oneself (gongyo and daimoku) and for others (sharing the teaching); studying Nichiren’s writings deepens and strengthens faith.',
    ],
    ja: '仏道修行の三つの基本。信心が根本であり、行には自行（勤行・唱題）と化他行（弘教）があり、学（御書などの教学の研鑽）は信心を深め強める。',
    see: ['shinjin', 'gosho', 'shakubuku'],
  },
  {
    id: 'shakubuku',
    cat: 'practice',
    jp: '折伏',
    kana: 'しゃくぶく',
    it: [
      'Shakubuku',
      'Diffondere l’insegnamento',
      'Letteralmente «rompere e sottomettere» (le opinioni errate). Indica il dialogo con cui si condivide con gli altri il Buddismo e il valore di Nam-myoho-renge-kyo, con il desiderio sincero della loro felicità. Nichiren insegnò che nell’epoca di Mappo è il metodo appropriato; si affianca allo shoju.',
    ],
    en: [
      'Shakubuku',
      'Sharing the teaching',
      'Literally "to break and subdue" (mistaken views). It is the dialogue through which one shares Buddhism and the value of Nam-myoho-renge-kyo with others, sincerely wishing for their happiness. Nichiren taught it is the appropriate method in the Latter Day of the Law; its counterpart is shoju.',
    ],
    ja: '「破折屈伏」の意。相手の幸福を真剣に願い、対話を通して仏法と南無妙法蓮華経の偉大さを伝えること。大聖人は末法には折伏が適していると説いた。摂受と対をなす。',
    see: ['shoju', 'kosen-rufu', 'mappo', 'fukyo'],
  },
  {
    id: 'shoju',
    cat: 'practice',
    jp: '摂受',
    kana: 'しょうじゅ',
    it: [
      'Shoju',
      'Accogliere',
      'Letteralmente «accogliere e abbracciare»: diffondere il Buddismo accettando gradualmente le opinioni dell’altro, senza confutarle. È il metodo complementare allo shakubuku; quale usare dipende dal tempo e dalle persone.',
    ],
    en: [
      'Shoju',
      'Embracing',
      'Literally "to embrace and accept": spreading Buddhism by gradually leading others while accepting their views, without refuting them. It is the method complementary to shakubuku; which one to use depends on the time and the people.',
    ],
    ja: '「摂引容受」の意。相手の考えをすぐには否定せず、受け入れながら次第に導いていく弘教の方法。折伏と対をなし、時と相手によって用いる。',
    see: ['shakubuku'],
  },
  {
    id: 'kosen-rufu',
    cat: 'practice',
    jp: '広宣流布',
    kana: 'こうせんるふ',
    it: [
      'Kosen-rufu',
      'Ampia propagazione',
      'Letteralmente «dichiarare e diffondere ampiamente», espressione tratta dal Sutra del Loto. Indica la diffusione della Legge mistica nella società e, con essa, la realizzazione della pace e della felicità per tutte le persone. Non è un traguardo fisso, ma un processo continuo.',
    ],
    en: [
      'Kosen-rufu',
      'Widespread propagation',
      'Literally "to widely declare and spread", an expression from the Lotus Sutra. It means spreading the Mystic Law in society and, through it, realising peace and happiness for all people. It is not a fixed goal but an ongoing process.',
    ],
    ja: '法華経に由来する言葉で、「広く宣べて流布する」の意。妙法を社会に広め、それによって万人の幸福と世界の平和を実現していくこと。到達点ではなく、絶えず続く流れである。',
    see: ['shakubuku', 'jiyu-bosatsu', 'rissho-ankoku'],
  },
  {
    id: 'zadankai',
    cat: 'practice',
    jp: '座談会',
    kana: 'ざだんかい',
    it: [
      'Zadankai',
      'Riunione di discussione',
      'Piccolo incontro, spesso in una casa, in cui i praticanti e i loro amici recitano insieme, condividono esperienze e studiano il Buddismo. È al centro delle attività della Soka Gakkai fin dalla sua fondazione.',
    ],
    en: [
      'Zadankai',
      'Discussion meeting',
      'A small gathering, often in a home, where practitioners and their friends chant together, share experiences and study Buddhism. It has been at the heart of Soka Gakkai activities since its founding.',
    ],
    ja: '会員や友人が地域の家庭などに集い、共に唱題し、体験を語り合い、仏法を学ぶ小さな会合。創価学会の創立以来、活動の中心となっている。',
    see: ['soka-gakkai', 'itai-doshin'],
  },
  {
    id: 'ichinen',
    cat: 'practice',
    jp: '一念',
    kana: 'いちねん',
    it: [
      'Ichinen',
      'Singolo istante di vita',
      'La vita in ogni singolo istante, che contiene tutti i Dieci mondi e i tremila regni. Nella pratica indica l’atteggiamento interiore, la determinazione profonda con cui si affronta ogni cosa: cambiando il proprio ichinen cambia anche il modo in cui la vita e l’ambiente rispondono.',
    ],
    en: [
      'Ichinen',
      'Single moment of life',
      'Life at each single moment, which contains all Ten Worlds and the three thousand realms. In practice it means one’s inner attitude, the deep determination with which one faces things: when one’s ichinen changes, the way one’s life and environment respond changes too.',
    ],
    ja: '瞬間瞬間の生命のこと。その一念に十界・三千の諸法が具わる。実践の上では、物事に臨む心の姿勢、根本の決意を指し、一念が変われば自身の生命も環境も変わっていく。',
    see: ['ichinen-sanzen', 'esho-funi', 'jikkai'],
  },
  // --------------------------------------------------------------- doctrine
  {
    id: 'jikkai',
    cat: 'doctrine',
    jp: '十界',
    kana: 'じっかい',
    it: [
      'Dieci mondi',
      'Jikkai',
      'Dieci stati vitali che ogni persona sperimenta: Inferno, Avidità, Animalità, Collera, Umanità (o Tranquillità), Estasi (o Cielo), Apprendimento, Realizzazione, Bodhisattva e Buddità. Non sono luoghi, ma condizioni della vita che cambiano di momento in momento.',
    ],
    en: [
      'Ten Worlds',
      'Jikkai',
      'Ten states of life that every person experiences: Hell, Hunger, Animality, Anger, Humanity (or Tranquillity), Heaven (or Rapture), Learning, Realisation, Bodhisattva and Buddhahood. They are not places but conditions of life that change from moment to moment.',
    ],
    ja: '生命の十種の境涯。地獄・餓鬼・畜生・修羅・人・天・声聞・縁覚・菩薩・仏の十界をいう。場所ではなく、瞬間瞬間に変化する生命の状態である。',
    see: ['jikkai-gogu', 'ichinen-sanzen', 'buddhahood'],
  },
  {
    id: 'jikkai-gogu',
    cat: 'doctrine',
    jp: '十界互具',
    kana: 'じっかいごぐ',
    it: [
      'Mutuo possesso dei dieci mondi',
      'Jikkai gogu',
      'Ognuno dei Dieci mondi contiene in sé tutti gli altri. Per questo anche una persona che soffre ha in sé la Buddità, e la Buddità non esiste separata dagli altri nove mondi: ci si illumina così come si è, senza dover diventare un’altra persona.',
    ],
    en: [
      'Mutual possession of the Ten Worlds',
      'Jikkai gogu',
      'Each of the Ten Worlds contains all the others. So even a person who is suffering has Buddhahood within, and Buddhahood does not exist apart from the other nine worlds: one attains enlightenment as one is, without becoming someone else.',
    ],
    ja: '十界のそれぞれが十界を具えていること。苦しんでいる人の生命にも仏界があり、仏界も九界を離れてあるのではない。ゆえに、ありのままの姿で成仏できる。',
    see: ['jikkai', 'ichinen-sanzen', 'sokushin-jobutsu'],
  },
  {
    id: 'junyoze',
    cat: 'doctrine',
    jp: '十如是',
    kana: 'じゅうにょぜ',
    it: [
      'Dieci fattori',
      'Junyoze',
      'Dieci aspetti comuni a ogni forma di vita, esposti nel capitolo Hoben del Sutra del Loto: aspetto, natura, entità, potere, influenza, causa interna, relazione, effetto latente, effetto manifesto e coerenza dall’inizio alla fine.',
    ],
    en: [
      'Ten factors',
      'Junyoze',
      'Ten aspects shared by every form of life, set out in the "Expedient Means" chapter of the Lotus Sutra: appearance, nature, entity, power, influence, internal cause, relationship, latent effect, manifest effect, and consistency from beginning to end.',
    ],
    ja: '法華経方便品に説かれる、あらゆる生命に共通する十の側面。如是相・如是性・如是体・如是力・如是作・如是因・如是縁・如是果・如是報・如是本末究竟等をいう。',
    see: ['ichinen-sanzen', 'gongyo'],
  },
  {
    id: 'san-seken',
    cat: 'doctrine',
    jp: '三世間',
    kana: 'さんせけん',
    it: [
      'Tre regni',
      'San seken',
      'I tre ambiti in cui si manifesta la vita: il regno dei cinque componenti (forma, percezione, concezione, volizione e coscienza), il regno degli esseri viventi e il regno dell’ambiente.',
    ],
    en: [
      'Three realms',
      'San seken',
      'The three dimensions in which life manifests itself: the realm of the five components (form, perception, conception, volition and consciousness), the realm of living beings, and the realm of the environment.',
    ],
    ja: '生命が現れる三つの次元。五陰世間（色・受・想・行・識）、衆生世間、国土世間をいう。',
    see: ['ichinen-sanzen', 'esho-funi'],
  },
  {
    id: 'ichinen-sanzen',
    cat: 'doctrine',
    jp: '一念三千',
    kana: 'いちねんさんぜん',
    it: [
      'Ichinen sanzen',
      'Tremila regni in un singolo istante di vita',
      'Principio formulato da T’ien-t’ai (Zhiyi) sulla base del Sutra del Loto: dieci mondi, che si contengono a vicenda (cento), per i dieci fattori (mille), per i tre regni (tremila). Significa che ogni istante di vita contiene tutti i fenomeni, e che cambiando la propria vita interiore si può trasformare anche l’ambiente.',
    ],
    en: [
      'Ichinen sanzen',
      'Three thousand realms in a single moment of life',
      'A principle formulated by T’ien-t’ai (Zhiyi) on the basis of the Lotus Sutra: ten worlds, each containing the others (a hundred), times the ten factors (a thousand), times the three realms (three thousand). It means that every moment of life contains all phenomena, and that by changing one’s inner life one can transform one’s environment too.',
    ],
    ja: '天台大師が法華経に基づいて立てた法門。十界が互いに十界を具えて百界、それぞれに十如是を具えて千如是、さらに三世間を具えて三千となる。一瞬の生命に一切の法が具わり、自身の一念の変革が環境をも変えていくことを示す。',
    see: ['ichinen', 'jikkai-gogu', 'junyoze', 'san-seken', 'tiantai'],
  },
  {
    id: 'buddhahood',
    cat: 'doctrine',
    jp: '仏界',
    kana: 'ぶっかい',
    it: [
      'Buddità',
      'Bukkai',
      'Il più alto dei Dieci mondi: uno stato di vita di saggezza, coraggio, compassione e pienezza, che non viene scosso dalle difficoltà. Esiste in ogni persona e si manifesta recitando Nam-myoho-renge-kyo.',
    ],
    en: [
      'Buddhahood',
      'Bukkai',
      'The highest of the Ten Worlds: a state of life of wisdom, courage, compassion and fulfilment that difficulties cannot shake. It exists in every person and is brought forth by chanting Nam-myoho-renge-kyo.',
    ],
    ja: '十界の最高の境涯。智慧・勇気・慈悲にあふれ、どんな困難にも揺るがない生命の状態。万人に具わり、南無妙法蓮華経の唱題によって涌現する。',
    see: ['jikkai', 'sokushin-jobutsu', 'gohonzon'],
  },
  {
    id: 'sokushin-jobutsu',
    cat: 'doctrine',
    jp: '即身成仏',
    kana: 'そくしんじょうぶつ',
    it: [
      'Illuminazione in questa esistenza',
      'Sokushin jobutsu',
      'Raggiungere la Buddità in questa vita, così come si è, senza dover attraversare innumerevoli esistenze di pratica. È uno degli insegnamenti centrali del Sutra del Loto, che Nichiren rese accessibile a tutti attraverso il daimoku.',
    ],
    en: [
      'Attaining Buddhahood in this lifetime',
      'Sokushin jobutsu',
      'Attaining Buddhahood in this life, just as one is, without having to go through countless lifetimes of practice. It is a central teaching of the Lotus Sutra, which Nichiren made open to everyone through daimoku.',
    ],
    ja: '歴劫修行によらず、凡夫のその身のままで、この一生のうちに成仏すること。法華経の重要な法理であり、大聖人は唱題によって万人に開いた。',
    see: ['buddhahood', 'jikkai-gogu', 'bonno-soku-bodai'],
  },
  {
    id: 'ningen-kakumei',
    cat: 'doctrine',
    jp: '人間革命',
    kana: 'にんげんかくめい',
    it: [
      'Rivoluzione umana',
      'Ningen kakumei',
      'La trasformazione interiore di una persona, che sviluppa la propria Buddità e supera i propri limiti. Il concetto fu espresso da Josei Toda e sviluppato da Daisaku Ikeda: la rivoluzione umana di un singolo individuo può contribuire a cambiare il destino della società e dell’umanità.',
    ],
    en: [
      'Human revolution',
      'Ningen kakumei',
      'A person’s inner transformation, developing their Buddhahood and going beyond their limits. The idea was put forward by Josei Toda and developed by Daisaku Ikeda: the human revolution of a single individual can help change the destiny of society and of humankind.',
    ],
    ja: '自身の仏界を開き、自己の限界を超えていく内面の変革。戸田城聖が提唱し、池田大作が展開した。一人の人間革命が、社会と人類の宿命の転換にもつながっていくとされる。',
    see: ['shukumei-tenkan', 'soka-gakkai', 'ichinen'],
  },
  {
    id: 'karma',
    cat: 'doctrine',
    jp: '業',
    kana: 'ごう',
    it: [
      'Karma',
      'Go',
      'Letteralmente «azione». L’insieme delle cause create con pensieri, parole e azioni, che restano nella vita e producono effetti in futuro. Nel Buddismo di Nichiren il karma non è un destino immutabile: può essere trasformato.',
    ],
    en: [
      'Karma',
      'Go',
      'Literally "action". The causes created through thoughts, words and deeds, which remain in one’s life and bring about effects in the future. In Nichiren Buddhism karma is not an unchangeable fate: it can be transformed.',
    ],
    ja: '「行為」の意。身・口・意によってつくられた因が生命に刻まれ、未来に果報をもたらすこと。日蓮仏法では、業は変えられない運命ではなく、転換できるものとされる。',
    see: ['shukumei-tenkan', 'inga-guji'],
  },
  {
    id: 'shukumei-tenkan',
    cat: 'doctrine',
    jp: '宿命転換',
    kana: 'しゅくめいてんかん',
    it: [
      'Trasformare il karma',
      'Shukumei tenkan',
      'Cambiare il proprio karma, anche quello più pesante, attraverso la fede e la pratica, invece di subirlo. Si accompagna all’idea di «trasformare il karma in missione»: le difficoltà diventano l’occasione per mostrare agli altri il potere della Legge.',
    ],
    en: [
      'Changing karma',
      'Shukumei tenkan',
      'Transforming one’s karma, even the heaviest, through faith and practice, instead of being subject to it. It goes with the idea of "transforming karma into mission": difficulties become the occasion to show others the power of the Law.',
    ],
    ja: '信心と実践によって、どんなに重い宿業をも転換していくこと。「宿命を使命に変える」ともいわれ、苦難は妙法の力を示す機会となる。',
    see: ['karma', 'ningen-kakumei'],
  },
  {
    id: 'esho-funi',
    cat: 'doctrine',
    jp: '依正不二',
    kana: 'えしょうふに',
    it: [
      'Non dualità di vita e ambiente',
      'Esho funi',
      '«Sho» è il soggetto vivente, «e» l’ambiente in cui vive. Sono due aspetti distinti ma inseparabili di un’unica realtà: per questo trasformando sé stessi si trasforma anche il proprio ambiente.',
    ],
    en: [
      'Oneness of life and environment',
      'Esho funi',
      '"Sho" is the living subject, "e" the environment in which it lives. They are two distinct but inseparable aspects of a single reality: so by transforming oneself one also transforms one’s environment.',
    ],
    ja: '「正報」は生命の主体、「依報」はその主体が依って立つ環境。二つでありながら不可分の関係にある。ゆえに自身が変われば環境も変わる。',
    see: ['shiki-shin-funi', 'ichinen-sanzen', 'san-seken'],
  },
  {
    id: 'shiki-shin-funi',
    cat: 'doctrine',
    jp: '色心不二',
    kana: 'しきしんふに',
    it: [
      'Non dualità di corpo e mente',
      'Shiki shin funi',
      '«Shiki» è l’aspetto fisico della vita, «shin» quello spirituale. Sono inseparabili: la condizione della mente influisce sul corpo e viceversa.',
    ],
    en: [
      'Oneness of body and mind',
      'Shiki shin funi',
      '"Shiki" is the physical aspect of life, "shin" the spiritual one. They are inseparable: the state of the mind affects the body, and the other way round.',
    ],
    ja: '「色法」は生命の物質的・肉体的側面、「心法」は精神的側面。両者は分かちがたく、心の状態は身体に影響し、身体の状態も心に影響する。',
    see: ['esho-funi'],
  },
  {
    id: 'bonno-soku-bodai',
    cat: 'doctrine',
    jp: '煩悩即菩提',
    kana: 'ぼんのうそくぼだい',
    it: [
      'I desideri terreni sono illuminazione',
      'Bonno soku bodai',
      'I desideri e le sofferenze non vanno estirpati per raggiungere l’illuminazione: attraverso la pratica diventano proprio la spinta e il materiale da cui nascono la saggezza e la felicità.',
    ],
    en: [
      'Earthly desires are enlightenment',
      'Bonno soku bodai',
      'Desires and sufferings need not be eliminated to attain enlightenment: through practice they become the very energy and material from which wisdom and happiness arise.',
    ],
    ja: '煩悩を断じて悟りを得るのではなく、信心によって煩悩がそのまま菩提（悟りの智慧）へと転じていくこと。悩みこそが智慧と幸福の源泉となる。',
    see: ['shoji-soku-nehan', 'sokushin-jobutsu'],
  },
  {
    id: 'shoji-soku-nehan',
    cat: 'doctrine',
    jp: '生死即涅槃',
    kana: 'しょうじそくねはん',
    it: [
      'Le sofferenze di nascita e morte sono nirvana',
      'Shoji soku nehan',
      'Il nirvana, la libertà dalla sofferenza, non si trova fuggendo dal ciclo di nascita e morte, ma vivendolo con la Buddità: le stesse sofferenze della vita diventano la base di una condizione di pace e gioia.',
    ],
    en: [
      'The sufferings of birth and death are nirvana',
      'Shoji soku nehan',
      'Nirvana, freedom from suffering, is not found by escaping the cycle of birth and death, but by living it with Buddhahood: life’s very sufferings become the basis of a state of peace and joy.',
    ],
    ja: '生死の苦しみを離れて涅槃があるのではなく、仏界の生命で生死を生きるとき、その苦しみがそのまま安らぎと歓喜の境涯となること。',
    see: ['bonno-soku-bodai'],
  },
  {
    id: 'inga-guji',
    cat: 'doctrine',
    jp: '因果倶時',
    kana: 'いんがぐじ',
    it: [
      'Simultaneità di causa ed effetto',
      'Inga guji',
      'Causa ed effetto esistono insieme nello stesso istante, come nel fiore di loto, che porta fiori e semi nello stesso momento. Recitando Nam-myoho-renge-kyo si crea la causa della Buddità e, nello stesso istante, se ne manifesta l’effetto.',
    ],
    en: [
      'Simultaneity of cause and effect',
      'Inga guji',
      'Cause and effect exist together in the same moment, as in the lotus, which bears flowers and seeds at the same time. Chanting Nam-myoho-renge-kyo creates the cause for Buddhahood and, in that same moment, its effect appears.',
    ],
    ja: '因と果が同時に具わること。花と実が同時に具わる蓮華がその譬え。南無妙法蓮華経と唱えるその瞬間に、成仏の因と果が同時に具わる。',
    see: ['nmrk', 'karma'],
  },
  {
    id: 'santai',
    cat: 'doctrine',
    jp: '三諦',
    kana: 'さんたい',
    it: [
      'Tre verità',
      'Santai',
      'Tre aspetti di ogni fenomeno: la verità della non sostanzialità (ku: nulla ha un’esistenza fissa e separata), la verità dell’esistenza temporanea (ke: ogni cosa appare in forme concrete e mutevoli) e la verità della Via di mezzo (chu: l’essenza che le comprende entrambe).',
    ],
    en: [
      'Three truths',
      'Santai',
      'Three aspects of every phenomenon: the truth of non-substantiality (ku: nothing has a fixed, separate existence), the truth of temporary existence (ke: everything appears in concrete, changing forms) and the truth of the Middle Way (chu: the essence that embraces both).',
    ],
    ja: 'あらゆる事物に具わる三つの真理。空諦（固定的な実体がないこと）、仮諦（縁によって仮に姿・形を現していること）、中諦（その両面を具えた本質）をいう。',
    see: ['ichinen-sanzen', 'tiantai'],
  },
  {
    id: 'kushiki',
    cat: 'doctrine',
    jp: '九識',
    kana: 'くしき',
    it: [
      'Nove coscienze',
      'Kushiki',
      'I livelli della coscienza: i cinque sensi, la mente (sesta), la coscienza mano (settima, il senso dell’io), la coscienza alaya (ottava, dove si depositano gli effetti del karma) e la coscienza amala (nona), la coscienza fondamentalmente pura, che coincide con la Buddità.',
    ],
    en: [
      'Nine consciousnesses',
      'Kushiki',
      'The levels of consciousness: the five senses, the mind (sixth), the mano-consciousness (seventh, the sense of self), the alaya-consciousness (eighth, where the effects of karma are stored) and the amala-consciousness (ninth), the fundamentally pure consciousness that is one with Buddhahood.',
    ],
    ja: '生命の識の九つの層。眼・耳・鼻・舌・身の五識、第六の意識、自我に執着する第七の末那識、業を蔵する第八の阿頼耶識、そして根本清浄の第九の阿摩羅識（九識心王真如の都）をいう。',
    see: ['karma', 'buddhahood'],
  },
  {
    id: 'sansho-shima',
    cat: 'doctrine',
    jp: '三障四魔',
    kana: 'さんしょうしま',
    it: [
      'Tre ostacoli e quattro demoni',
      'Sansho shima',
      'Le funzioni che ostacolano la pratica. I tre ostacoli: dei desideri terreni, del karma e della retribuzione. I quattro demoni: dei cinque componenti, dei desideri terreni, della morte e il re demone del sesto cielo. Nichiren insegna che si presentano proprio quando la pratica progredisce, e che riconoscerli è il modo di superarli.',
    ],
    en: [
      'Three obstacles and four devils',
      'Sansho shima',
      'The functions that hinder practice. The three obstacles: earthly desires, karma and retribution. The four devils: the hindrance of the five components, of earthly desires, of death, and the devil king of the sixth heaven. Nichiren teaches that they appear precisely when practice advances, and that recognising them is the way to overcome them.',
    ],
    ja: '仏道修行を妨げる働き。三障は煩悩障・業障・報障、四魔は陰魔・煩悩魔・死魔・天子魔をいう。大聖人は、信心が進むときに必ず競い起こると教え、その正体を見破ることが勝利の道であるとした。',
    see: ['shiriki', 'shoten-zenjin'],
  },
  {
    id: 'shiriki',
    cat: 'doctrine',
    jp: '四力',
    kana: 'しりき',
    it: [
      'Quattro poteri',
      'Shiriki',
      'Il potere del Budda e il potere della Legge, contenuti nel Gohonzon, si manifestano grazie al potere della fede e al potere della pratica di chi recita. La forza della fede e della pratica è la chiave che li fa emergere.',
    ],
    en: [
      'Four powers',
      'Shiriki',
      'The power of the Buddha and the power of the Law, contained in the Gohonzon, are brought forth by the power of faith and the power of practice of the person who chants. The strength of faith and practice is the key that draws them out.',
    ],
    ja: '仏力・法力・信力・行力のこと。御本尊に具わる仏力・法力は、唱える人の信力・行力によって現れる。',
    see: ['shinjin', 'gohonzon', 'kudoku'],
  },
  {
    id: 'kudoku',
    cat: 'doctrine',
    jp: '功徳',
    kana: 'くどく',
    it: [
      'Beneficio',
      'Kudoku',
      'Il risultato positivo della pratica. Nichiren spiega che «ku» significa eliminare il male e «toku» far emergere il bene. Esistono benefici evidenti, che si manifestano chiaramente, e benefici non evidenti, che si accumulano poco a poco e rendono la vita sempre più solida.',
    ],
    en: [
      'Benefit',
      'Kudoku',
      'The positive result of practice. Nichiren explains that "ku" means eliminating evil and "toku" bringing forth good. There are conspicuous benefits, which appear clearly, and inconspicuous ones, which build up little by little and make one’s life ever more solid.',
    ],
    ja: '信心の実践によって得られる福徳。大聖人は「功」は悪を滅すること、「徳」は善を生ずることと説いた。目に見えて現れる顕益と、目に見えず徐々に積み重なり生命を強くしていく冥益とがある。',
    see: ['shiriki', 'kyochi-myogo'],
  },
  {
    id: 'kyochi-myogo',
    cat: 'doctrine',
    jp: '境智冥合',
    kana: 'きょうちみょうごう',
    it: [
      'Fusione di realtà e saggezza',
      'Kyochi myogo',
      '«Kyo» è la realtà oggettiva, la Legge; «chi» è la saggezza soggettiva che la percepisce. Recitando al Gohonzon, la vita del praticante (saggezza) si fonde con la Legge (realtà), e si manifesta la Buddità.',
    ],
    en: [
      'Fusion of reality and wisdom',
      'Kyochi myogo',
      '"Kyo" is objective reality, the Law; "chi" is the subjective wisdom that perceives it. In chanting to the Gohonzon, the practitioner’s life (wisdom) fuses with the Law (reality), and Buddhahood appears.',
    ],
    ja: '「境」は客観の真理（法）、「智」はそれを悟る主観の智慧。御本尊（境）に向かって題目を唱えるとき、自身の生命（智）が冥合し、仏界が涌現する。',
    see: ['gohonzon', 'daimoku'],
  },
  {
    id: 'itai-doshin',
    cat: 'doctrine',
    jp: '異体同心',
    kana: 'いたいどうしん',
    it: [
      'Diversi corpi, stessa mente',
      'Itai doshin',
      'L’unità di persone diverse, ciascuna con la propria individualità, unite dallo stesso obiettivo e dalla stessa fede. Nichiren insegnò che con questo spirito si può realizzare qualsiasi cosa, mentre «stessi corpi, mente diversa» non porta a nulla.',
    ],
    en: [
      'Many in body, one in mind',
      'Itai doshin',
      'The unity of different people, each with their own individuality, joined by the same aim and the same faith. Nichiren taught that with this spirit anything can be achieved, whereas being "one in body, different in mind" achieves nothing.',
    ],
    ja: '一人ひとりの個性（異体）を生かしながら、同じ目的と信心（同心）で団結すること。大聖人は、異体同心であれば万事を成就できるが、同体異心では何事も成し遂げられないと教えた。',
    see: ['zadankai', 'kosen-rufu'],
  },
  {
    id: 'shitei-funi',
    cat: 'doctrine',
    jp: '師弟不二',
    kana: 'していふに',
    it: [
      'Non dualità di maestro e discepolo',
      'Shitei funi',
      'Il rapporto in cui il discepolo fa propri il cuore e l’impegno del maestro e li porta avanti con la propria vita. Nel Buddismo il maestro è chi mostra con l’esempio la via per realizzare la Buddità.',
    ],
    en: [
      'Oneness of mentor and disciple',
      'Shitei funi',
      'The relationship in which the disciple makes the mentor’s heart and commitment their own and carries them forward with their own life. In Buddhism the mentor is the one who shows by example the way to reveal Buddhahood.',
    ],
    ja: '弟子が師匠の心と誓願をわが心として、自らの人生で実践していくこと。仏法における師匠とは、成仏の道を自らの姿で示す人である。',
    see: ['soka-gakkai', 'ningen-kakumei'],
  },
  {
    id: 'jiyu-bosatsu',
    cat: 'doctrine',
    jp: '地涌の菩薩',
    kana: 'じゆのぼさつ',
    it: [
      'Bodhisattva della Terra',
      'Jiyu no bosatsu',
      'Gli innumerevoli bodhisattva che nel quindicesimo capitolo del Sutra del Loto emergono dalla terra, guidati dal bodhisattva Pratiche Superiori (Jogyo), e ai quali Shakyamuni affida la diffusione della Legge dopo la sua morte. Chi oggi pratica e diffonde Nam-myoho-renge-kyo è considerato tale.',
    ],
    en: [
      'Bodhisattvas of the Earth',
      'Jiyu no bosatsu',
      'The countless bodhisattvas who, in the fifteenth chapter of the Lotus Sutra, emerge from the earth led by Bodhisattva Superior Practices (Jogyo), and to whom Shakyamuni entrusts spreading the Law after his death. Those who practise and share Nam-myoho-renge-kyo today are seen as such.',
    ],
    ja: '法華経従地涌出品第十五で大地から涌き出た無数の菩薩。上行菩薩を上首とし、釈尊から滅後の弘教を託された。現代において南無妙法蓮華経を実践し弘める人々を指す。',
    see: ['kosen-rufu', 'lotus-sutra'],
  },
  {
    id: 'fukyo',
    cat: 'doctrine',
    jp: '不軽菩薩',
    kana: 'ふきょうぼさつ',
    it: [
      'Bodhisattva Mai Sprezzante',
      'Fukyo',
      'Il bodhisattva del ventesimo capitolo del Sutra del Loto che si inchinava davanti a ogni persona, riconoscendo in ciascuno la natura di Budda, anche quando veniva deriso e aggredito. È il modello del rispetto per la dignità di ogni vita.',
    ],
    en: [
      'Bodhisattva Never Disparaging',
      'Fukyo',
      'The bodhisattva of the twentieth chapter of the Lotus Sutra who bowed to every person, recognising the Buddha nature in each, even when mocked and attacked. He is the model of respect for the dignity of every life.',
    ],
    ja: '法華経常不軽菩薩品第二十に説かれる菩薩。悪口や迫害を受けても、すべての人に仏性があるとして礼拝し続けた。あらゆる生命の尊厳を敬う実践の模範である。',
    see: ['shakubuku', 'jikkai-gogu'],
  },
  {
    id: 'shoten-zenjin',
    cat: 'doctrine',
    jp: '諸天善神',
    kana: 'しょてんぜんじん',
    it: [
      'Divinità celesti',
      'Shoten zenjin',
      'Le funzioni protettive presenti nella vita e nell’ambiente, che sostengono e proteggono chi pratica la Legge. Non sono esseri esterni da venerare, ma forze che si attivano grazie alla fede.',
    ],
    en: [
      'Heavenly deities',
      'Shoten zenjin',
      'The protective functions present in life and in the environment, which support and protect those who practise the Law. They are not outside beings to worship, but forces activated through faith.',
    ],
    ja: '妙法を持つ人を守り支える、生命と環境に具わる働き。外にある崇拝の対象ではなく、信心によって発動する力である。',
    see: ['sansho-shima', 'esho-funi'],
  },
  {
    id: 'sandai-hiho',
    cat: 'doctrine',
    jp: '三大秘法',
    kana: 'さんだいひほう',
    it: [
      'Tre grandi leggi segrete',
      'Sandai hiho',
      'Il nucleo dell’insegnamento di Nichiren: l’oggetto di culto dell’insegnamento essenziale (il Gohonzon), il daimoku dell’insegnamento essenziale (Nam-myoho-renge-kyo) e il santuario dell’insegnamento essenziale (il luogo in cui si custodisce il Gohonzon e si pratica).',
    ],
    en: [
      'Three Great Secret Laws',
      'Sandai hiho',
      'The core of Nichiren’s teaching: the object of devotion of the essential teaching (the Gohonzon), the daimoku of the essential teaching (Nam-myoho-renge-kyo), and the sanctuary of the essential teaching (the place where the Gohonzon is enshrined and one practises).',
    ],
    ja: '日蓮仏法の根幹。本門の本尊（御本尊）、本門の題目（南無妙法蓮華経）、本門の戒壇（御本尊を安置し唱題する場所）の三つをいう。',
    see: ['gohonzon', 'daimoku'],
  },
  {
    id: 'sansho',
    cat: 'doctrine',
    jp: '三証',
    kana: 'さんしょう',
    it: [
      'Tre prove',
      'Sansho',
      'I tre criteri per valutare un insegnamento religioso: la prova documentale (è fondato sulle scritture?), la prova teorica (è ragionevole?) e la prova concreta (cambia davvero la vita delle persone?). Nichiren dava importanza soprattutto a quest’ultima.',
    ],
    en: [
      'Three proofs',
      'Sansho',
      'The three criteria for judging a religious teaching: documentary proof (is it based on the scriptures?), theoretical proof (is it reasonable?) and actual proof (does it really change people’s lives?). Nichiren gave the most weight to the last.',
    ],
    ja: '宗教の正邪を判定する三つの基準。文証（経文の裏付け）、理証（道理にかなうこと）、現証（実際の生活に現れる証拠）をいう。大聖人は特に現証を重んじた。',
    see: ['kudoku'],
  },
  {
    id: 'mappo',
    cat: 'doctrine',
    jp: '末法',
    kana: 'まっぽう',
    it: [
      'Ultimo giorno della Legge',
      'Mappo',
      'L’epoca, iniziata secondo la cronologia seguita da Nichiren intorno all’anno 1052, in cui gli insegnamenti di Shakyamuni perdono il potere di condurre le persone all’illuminazione. Nichiren insegnò che in quest’epoca la via è Nam-myoho-renge-kyo.',
    ],
    en: [
      'Latter Day of the Law',
      'Mappo',
      'The age, which by the chronology Nichiren followed began around 1052, in which Shakyamuni’s teachings lose the power to lead people to enlightenment. Nichiren taught that in this age the way is Nam-myoho-renge-kyo.',
    ],
    ja: '釈尊の教えが人々を成仏へ導く力を失う時代。大聖人が用いた年代観では1052年（永承7年）頃に始まる。大聖人は、末法には南無妙法蓮華経によって成仏できると説いた。',
    see: ['nichiren', 'shakubuku'],
  },
  // ---------------------------------------------------------------- history
  {
    id: 'lotus-sutra',
    cat: 'history',
    jp: '法華経',
    kana: 'ほけきょう',
    it: [
      'Sutra del Loto',
      'Hokekyo',
      'Uno dei sutra del Buddismo Mahayana, considerato da T’ien-t’ai e da Nichiren l’insegnamento più alto di Shakyamuni. Afferma che tutte le persone, senza distinzioni, possono diventare Budda. La traduzione cinese di Kumarajiva, in ventotto capitoli, si intitola Myoho-renge-kyo.',
    ],
    en: [
      'Lotus Sutra',
      'Hokekyo',
      'One of the sutras of Mahayana Buddhism, regarded by T’ien-t’ai and Nichiren as Shakyamuni’s highest teaching. It declares that all people, without distinction, can become Buddhas. Kumarajiva’s Chinese translation, in twenty-eight chapters, is titled Myoho-renge-kyo.',
    ],
    ja: '大乗仏教の経典の一つで、天台大師と日蓮大聖人が釈尊の最高の教えとした。万人成仏を説く。鳩摩羅什訳の『妙法蓮華経』は二十八品からなる。',
    see: ['nmrk', 'gongyo', 'shakyamuni', 'tiantai'],
  },
  {
    id: 'gosho',
    cat: 'history',
    jp: '御書',
    kana: 'ごしょ',
    it: [
      'Gosho',
      'Scritti di Nichiren Daishonin',
      'Le lettere e i trattati scritti da Nichiren Daishonin, molti dei quali indirizzati a singoli discepoli per incoraggiarli nelle loro difficoltà. Sono la base dello studio nel Buddismo di Nichiren.',
    ],
    en: [
      'Gosho',
      'Writings of Nichiren Daishonin',
      'The letters and treatises written by Nichiren Daishonin, many of them addressed to individual disciples to encourage them through their difficulties. They are the basis of study in Nichiren Buddhism.',
    ],
    ja: '日蓮大聖人が著した論文や手紙。その多くは、苦難の中にある門下一人ひとりを励ますために書かれた。日蓮仏法の教学の根本である。',
    see: ['nichiren', 'shin-gyo-gaku', 'rissho-ankoku'],
  },
  {
    id: 'rissho-ankoku',
    cat: 'history',
    jp: '立正安国論',
    kana: 'りっしょうあんこくろん',
    it: [
      'Rissho ankoku ron',
      'Adottare l’insegnamento corretto per assicurare la pace nel paese',
      'Il trattato che Nichiren presentò nel 1260 al reggente Hojo Tokiyori, in un Giappone colpito da carestie, epidemie e terremoti. Sostiene che la pace della società nasce dal rispetto per la vita fondato su un insegnamento corretto.',
    ],
    en: [
      'Rissho ankoku ron',
      'On Establishing the Correct Teaching for the Peace of the Land',
      'The treatise Nichiren submitted in 1260 to the regent Hojo Tokiyori, in a Japan struck by famine, epidemics and earthquakes. It holds that peace in society arises from respect for life founded on a correct teaching.',
    ],
    ja: '1260年（文応元年）、飢饉・疫病・大地震が相次ぐなか、日蓮大聖人が北条時頼に提出した国主諫暁の書。正しい法に基づく生命尊厳の思想こそが社会の平和の基盤であると説く。',
    see: ['nichiren', 'kosen-rufu', 'gosho'],
  },
  {
    id: 'shakyamuni',
    cat: 'history',
    jp: '釈尊',
    kana: 'しゃくそん',
    it: [
      'Shakyamuni',
      'Il Budda storico',
      'Siddhartha Gautama, il fondatore del Buddismo, vissuto in India circa 2500 anni fa. Lasciò la vita di principe per cercare una risposta alle sofferenze di nascita, vecchiaia, malattia e morte; dopo l’illuminazione insegnò per circa cinquant’anni. Il Sutra del Loto è considerato il suo insegnamento più alto.',
    ],
    en: [
      'Shakyamuni',
      'The historical Buddha',
      'Siddhartha Gautama, the founder of Buddhism, who lived in India about 2,500 years ago. He left his life as a prince to seek an answer to the sufferings of birth, old age, sickness and death; after his enlightenment he taught for about fifty years. The Lotus Sutra is regarded as his highest teaching.',
    ],
    ja: '仏教の開祖ゴータマ・シッダールタ。約2500年前のインドに生まれ、生老病死の苦しみの解決を求めて王子の地位を捨てて出家した。成道後、約50年にわたって法を説いた。法華経はその最高の教えとされる。',
    see: ['lotus-sutra', 'mappo'],
  },
  {
    id: 'tiantai',
    cat: 'history',
    jp: '天台大師',
    kana: 'てんだいだいし',
    it: [
      'T’ien-t’ai',
      'Zhiyi, 538–597',
      'Il grande maestro cinese, fondatore della scuola T’ien-t’ai, che sistematizzò gli insegnamenti di Shakyamuni ponendo al vertice il Sutra del Loto e formulò il principio di ichinen sanzen nella sua opera «Grande concentrazione e visione profonda».',
    ],
    en: [
      'T’ien-t’ai',
      'Zhiyi, 538–597',
      'The great Chinese teacher, founder of the T’ien-t’ai school, who classified Shakyamuni’s teachings placing the Lotus Sutra at the top and formulated the principle of ichinen sanzen in his work "Great Concentration and Insight".',
    ],
    ja: '中国・天台宗の開祖、智顗（538〜597年）。釈尊一代の教えを整理して法華経を最第一とし、『摩訶止観』で一念三千の法門を説いた。',
    see: ['ichinen-sanzen', 'lotus-sutra', 'santai'],
  },
  {
    id: 'nichiren',
    cat: 'history',
    jp: '日蓮大聖人',
    kana: 'にちれんだいしょうにん',
    it: [
      'Nichiren Daishonin',
      '1222–1282',
      'Il monaco giapponese che il 28 aprile 1253 proclamò per la prima volta Nam-myoho-renge-kyo come via perché tutte le persone possano raggiungere la Buddità. Per la sua predicazione e per le sue rimostranze alle autorità subì gravi persecuzioni, tra cui l’esilio sull’isola di Sado. «Daishonin» significa «grande saggio».',
    ],
    en: [
      'Nichiren Daishonin',
      '1222–1282',
      'The Japanese monk who on 28 April 1253 first proclaimed Nam-myoho-renge-kyo as the way for all people to attain Buddhahood. For his teaching and his remonstrations with the authorities he suffered severe persecutions, among them exile to the island of Sado. "Daishonin" means "great sage".',
    ],
    ja: '1222年、安房国に生まれる。1253年（建長5年）4月28日に立宗宣言し、万人成仏の道として南無妙法蓮華経を弘めた。弘教と国主諫暁のゆえに、竜の口の法難や佐渡流罪などの大難を受けた。1282年入滅。',
    see: ['nmrk', 'gosho', 'rissho-ankoku', 'nikko'],
  },
  {
    id: 'nikko',
    cat: 'history',
    jp: '日興上人',
    kana: 'にっこうしょうにん',
    it: [
      'Nikko Shonin',
      '1246–1333',
      'Il discepolo diretto e successore di Nichiren Daishonin, che lo seguì anche nell’esilio di Sado. Dopo la morte del maestro ne custodì fedelmente l’insegnamento, raccolse e copiò i suoi scritti e formò numerosi discepoli.',
    ],
    en: [
      'Nikko Shonin',
      '1246–1333',
      'Nichiren Daishonin’s direct disciple and successor, who followed him even into exile on Sado. After his teacher’s death he faithfully preserved the teaching, collected and copied his writings and raised many disciples.',
    ],
    ja: '日蓮大聖人の直弟子であり後継者（1246〜1333年）。佐渡流罪にも随従した。大聖人の入滅後、その教えを厳格に守り、御書を収集・書写し、多くの弟子を育てた。',
    see: ['nichiren', 'gosho'],
  },
  {
    id: 'soka-gakkai',
    cat: 'history',
    jp: '創価学会',
    kana: 'そうかがっかい',
    it: [
      'Soka Gakkai',
      'Società per la creazione di valore',
      'Associazione buddista laica fondata in Giappone nel 1930 dall’educatore Tsunesaburo Makiguchi e da Josei Toda, che pratica il Buddismo di Nichiren Daishonin. Il terzo presidente, Daisaku Ikeda, la diffuse nel mondo: nel 1975 nacque la Soka Gakkai Internazionale (SGI). In Italia è l’Istituto Buddista Italiano Soka Gakkai.',
    ],
    en: [
      'Soka Gakkai',
      'Value-Creating Society',
      'A lay Buddhist organisation founded in Japan in 1930 by the educator Tsunesaburo Makiguchi and by Josei Toda, practising the Buddhism of Nichiren Daishonin. Its third president, Daisaku Ikeda, spread it worldwide: Soka Gakkai International (SGI) was founded in 1975.',
    ],
    ja: '1930年、教育者・牧口常三郎と戸田城聖によって創立された、日蓮大聖人の仏法を信奉する在家の仏教団体。第三代会長・池田大作のもとで世界に広がり、1975年に創価学会インタナショナル（SGI）が発足した。',
    see: ['zadankai', 'ningen-kakumei', 'shitei-funi'],
  },
];
