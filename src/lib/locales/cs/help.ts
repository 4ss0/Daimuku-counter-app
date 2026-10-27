import type { HelpTexts } from '../types';

export default {
  voice: [
    'Než začneš: tvůj hlas',
    'Při prvním spuštění aplikace otevře „Tvůj hlas“. Vyber rychlost (Pomalu, Středně nebo Rychle), nastav, kolik daimoku zarecituješ, stiskni Nahrát, zarecituj přesně tolik daimoku a stiskni Hotovo. Pak klepni na Přidat: aplikace se z té nahrávky učí.\n\nUdělej aspoň jednu nahrávku v každé rychlosti. Další můžeš přidat kdykoli: pokud se počítání často mýlí, hodně pomůže ještě jedna nahrávka v tvém obvyklém tempu.',
  ],
  count: [
    'Počítání daimoku',
    'Na obrazovce Počítání stiskni Start a recituj. Číslo stoupne, jakmile aplikace uslyší závěrečné „kyo“ každého daimoku; tečka vedle stavu se rozsvítí, když tě slyší.\n\nAž skončíš, stiskni Stop: relace se uloží. Pokud aplikace některá vynechala nebo jich přidala moc, oprav číslo pomocí − a +: statistiky používají opravenou hodnotu. Relaci můžeš také smazat.',
  ],
  coins: [
    'Mince: dohledaná daimoku',
    'Kromě okamžitého rozpoznávání si aplikace každé dvě sekundy znovu a pozorněji poslechne poslední minutu. Když najde daimoku, která jí unikla (přehlušená zvonkem, korálky nebo hlukem), přidá je: vidíš je vlétat do počítadla jako mince.',
  ],
  screen: [
    'Se zhasnutou obrazovkou',
    'Obrazovku můžeš zhasnout nebo používat jiné aplikace: počítání pokračuje a oznámení ukazuje aktuální číslo. Když se do aplikace vrátíš, počítadlo už ukazuje správné číslo.\n\nV Nastavení si můžeš zvolit, aby obrazovka během počítání zůstala zapnutá. I když přejdeš na jinou obrazovku (Statistiky, Slovník…), relace pokračuje: horní lišta ukazuje počet, klepnutím na ni se vrátíš k počítadlu.',
  ],
  tips: [
    'Tipy pro přesné počítání',
    '• Drž telefon blízko sebe a nezakrývej mikrofon.\n• Vyslov každé daimoku celé, až po „kyo“: věta přerušená dřív se nezapočítá.\n• Hlasité, trvalé zvuky u telefonu (zvonek, třené korálky) mohou způsobit, že pár daimoku unikne: kontroly jich většinu dohledají.\n• Počítání funguje i ve skupině, ale je přesnější, když je telefonu nejblíž tvůj hlas.\n• Pokud se počítání často mýlí, přidej nahrávky svého hlasu.',
  ],
  stats: [
    'Statistiky, cíl a sdílení',
    'Ve Statistikách vidíš svá daimoku podle dní, týdnů, měsíců a let, svou sérii a průměr: klepni na bod grafu pro podrobnosti. V Nastavení vyber, které období se zobrazí jako první. Nastav si denní cíl: kruh na obrazovce Počítání se plní, dokud ho nesplníš.\n\nRecitoval(a) jsi bez aplikace, třeba na setkání? V „Relacích“ vyber den a relaci (ráno nebo večer) a napiš, kolik daimoku jsi zarecitoval(a). Ikonou sdílení můžeš sdílet relaci, období nebo splněný cíl.',
  ],
  goals: [
    'Cíle a připomínky',
    'V Nastavení → Cíle daimoku vyber denní cíl a podle chuti i týdenní, měsíční a roční. Denní cíl je rozdělen do dvou relací: ráno (do 14:00) a večer; obrazovka Počítání ukazuje, jak daleko jsi v každé z nich.\n\nPočítadlo pokračuje v aktuální relaci a začíná od už zarecitovaných daimoku. Chceš-li pokračovat v té druhé (třeba v ranním daimoku recitovaném odpoledne), klepni na ni před stisknutím Start. Každá uložená relace zůstane v ránu nebo večeru, které jsi zvolil(a).\n\nV nabídce ti Připomínky pošlou oznámení ve zvolený čas pro ranní i večerní relaci.',
  ],
  intents: [
    'Tvoje cíle',
    'V nabídce v sekci Cíle píšeš, za co recituješ: jeden cíl nebo více, vlastními slovy. Pod každým posouvej posuvník, abys ukázal(a), jak daleko jsi.\n\nAž přijde čas, klepni na Uzavřít a vyber důvod: cíle bylo dosaženo, nebo se našla lepší cesta. Uzavřené cíle zůstávají dole na stránce: můžeš si je znovu přečíst, znovu otevřít nebo smazat.',
  ],
  badges: [
    'Milníky a odpuštěný den',
    'Milník získáš při 100, 1 000, 10 000, 1 milionu, 5 a 10 milionech daimoku a za dny recitace v řadě: týden, měsíc, šest měsíců, pak 1, 2, 5, 10, 15 a 20 let. Všechny je najdeš v nabídce v Milnících, spolu s odpuštěným dnem.\n\nPokud jsi některý den nerecitoval(a), můžeš ho jednou týdně odpustit tlačítkem ve Statistikách: tvá série dní v řadě se nepřeruší.',
  ],
  bell: [
    'Zvonek a pozadí',
    'V nabídce si v sekci Zvonek můžeš vybrat ze tří zvonků, od nejvyššího po nejhlubší. Když zapneš jeho tlačítko, objeví se na obrazovce Počítání vedle Start zvonek: klepnout na něj je jako do něj udeřit. Můžeš na něj zvonit i během recitace: počítadlo ho neslyší, protože zatímco zní, odstraňuje jeho tóny z mikrofonu.\n\nV Nastavení → Pozadí vyber lotos nebo jednu z pěti ilustrací (Lotosová sútra, Buddha, chrám, příroda, třešňové květy), každou světlou nebo tmavou podle motivu, případně vlastní obrázek.',
  ],
  backup: [
    'Záloha a výměna telefonu',
    'V Nastavení v sekci „Tvoje data“ zapni Automatické ukládání: poprvé vybereš, kde má soubor být (v telefonu nebo na Drive), a pak aplikace sama ukládá vše po každé relaci, každé změně nahrávek hlasu a každém milníku. V novém telefonu nebo po přeinstalování aplikace zvol při prvním spuštění „Načíst uložená data“ a vyber ten soubor.\n\nV Nastavení v sekci „Tvoje data“ vytvoř zálohu: obsahuje tvé relace, cíl, nastavení a nahrávky hlasu. Uchovávej ji mimo telefon (Drive, e-mail) a v novém telefonu použij „Obnovit ze zálohy“. Relace můžeš také exportovat do souboru CSV.',
  ],
  menu: [
    'Nabídka',
    'Klepni na ☰ vlevo nahoře nebo přejeď doprava z levé poloviny obrazovky a otevřeš nabídku: počítadlo, cíle, Gongyo, milníky, připomínky, zvonek, slovník buddhistických pojmů, dary, tento návod a soukromí. Zavřeš ji přejetím doleva nebo klepnutím mimo ni.\n\nV Gongyo text stoupá vzhůru jako titulky na konci filmu a písmena se plní barvou, jak je recituješ, jako při karaoke: uprostřed dva řádky ke čtení, nad nimi dva už zarecitované, pod nimi ty následující. Rychlost nastavíš posuvníkem: když jím pohybuješ, vidíš náhled. „Překlad“ obsahuje obě kapitoly řádek po řádku ve zvoleném jazyce.\n\nPokaždé, když aplikaci otevřeš, čeká tě citát dne ze spisů Nichirena Daishonina, z Lotosové sútry nebo od prezidentů Soka Gakkai. Šipky (nebo přejetí) ukazují citáty jiných dní; „Vstoupit“ tě vezme do aplikace.',
  ],
  trouble: [
    'Když něco nefunguje',
    '• „Aplikace nemůže používat mikrofon“: povol mikrofon v nastavení Androidu (Aplikace → Daimoku Counter → Oprávnění).\n• Žádné oznámení se zhasnutou obrazovkou: povol oznámení aplikace.\n• Počítání se se zhasnutou obrazovkou zastaví: některé telefony zavírají aplikace kvůli úspoře baterie; vyřaď Daimoku Counter z optimalizace baterie.\n• Počítání se mýlí: po Stop klepni na „Uložit zvuk a nahlásit problém“ a pošli soubor vývojáři (kontakt je v úplných zásadách ochrany soukromí, na stránce Soukromí).',
  ],
} satisfies HelpTexts;
