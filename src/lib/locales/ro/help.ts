import type { HelpTexts } from '../types';

export default {
  voice: [
    'Înainte de a începe: vocea ta',
    'La prima pornire aplicația deschide „Vocea ta”. Alege o viteză (Lent, Mediu sau Rapid), setează câte daimoku vei recita, apasă Înregistrează, recită exact atâtea daimoku și apasă Gata. Apoi atinge Adaugă: aplicația învață din acea înregistrare.\n\nFă cel puțin o înregistrare la fiecare viteză. Poți adăuga altele oricând: dacă numărarea greșește des, încă o înregistrare în ritmul tău obișnuit ajută mult.',
  ],
  count: [
    'Numărarea daimoku',
    'Pe ecranul Numărare apasă Start și recită. Numărul crește imediat ce aplicația aude „kyo”-ul final al fiecărui daimoku; punctul de lângă stare se aprinde când te aude.\n\nCând ai terminat apasă Stop: sesiunea este salvată. Dacă aplicația a ratat unele sau a adăugat prea multe, corectează numărul cu − și +: statisticile folosesc valoarea corectată. Poți și șterge sesiunea.',
  ],
  coins: [
    'Monede: daimoku recuperate',
    'Pe lângă recunoașterea imediată, la fiecare două secunde aplicația ascultă din nou, mai atent, ultimul minut. Dacă găsește daimoku care i-au scăpat (acoperite de clopoțel, de mătănii sau de un zgomot), le adaugă: le vezi intrând în contor ca niște monede.',
  ],
  screen: [
    'Cu ecranul stins',
    'Poți stinge ecranul sau folosi alte aplicații: numărarea continuă și o notificare arată numărul actual. Când revii în aplicație, contorul arată deja numărul corect.\n\nÎn Setări poți alege să ții ecranul aprins în timpul numărării. Chiar dacă treci pe alt ecran (Statistici, Dicționar…), sesiunea continuă: bara de sus arată numărătoarea; atinge-o ca să revii la contor.',
  ],
  tips: [
    'Sfaturi pentru o numărare precisă',
    '• Ține telefonul aproape de tine, fără să acoperi microfonul.\n• Rostește fiecare daimoku complet, până la „kyo”: o frază întreruptă înainte nu este numărată.\n• Zgomotele puternice și continue lângă telefon (clopoțelul, mătăniile frecate) pot face să scape câteva daimoku: verificările le recuperează pe cele mai multe.\n• Numărarea funcționează și în grup, dar este mai precisă când vocea cea mai apropiată de telefon este a ta.\n• Dacă numărarea greșește des, adaugă înregistrări ale vocii tale.',
  ],
  stats: [
    'Statistici, obiectiv și distribuire',
    'În Statistici îți vezi daimoku pe zile, săptămâni, luni și ani, seria și media: atinge un punct din grafic pentru detalii. În Setări alege ce perioadă se afișează prima. Setează un obiectiv zilnic: inelul de pe ecranul Numărare se umple până îl atingi.\n\nAi recitat fără aplicație, de exemplu la o întâlnire? La „Sesiuni” alege ziua și sesiunea (dimineață sau seară) și scrie câte daimoku ai recitat. Cu pictograma de distribuire poți distribui o sesiune, o perioadă sau un obiectiv atins.',
  ],
  goals: [
    'Obiective și mementouri',
    'În Setări → Obiective alege obiectivul zilnic și, dacă vrei, unul săptămânal, lunar și anual. Obiectivul zilnic este împărțit în două sesiuni: dimineața (până la ora 14) și seara; ecranul Numărare arată cât ai avansat în fiecare.\n\nContorul continuă sesiunea curentă, pornind de la daimoku deja recitate. Pentru a o continua pe cealaltă (de exemplu daimoku de dimineață recitate după-amiaza), atinge-o înainte de a apăsa Start. Fiecare sesiune salvată rămâne în dimineața sau seara aleasă.\n\nÎn meniu, Mementouri îți trimite o notificare la ora aleasă pentru sesiunea de dimineață și cea de seară.',
  ],
  badges: [
    'Repere și ziua iertată',
    'Obții un reper la 100, 1.000, 10.000, 1 milion, 5 și 10 milioane de daimoku și pentru zilele de recitare la rând: o săptămână, o lună, șase luni, apoi 1, 2, 5, 10, 15 și 20 de ani. Le găsești pe toate în meniu, la Repere, împreună cu ziua iertată.\n\nDacă într-o zi nu ai recitat, o dată pe săptămână o poți ierta cu butonul din Statistici: seria ta de zile la rând nu se întrerupe.',
  ],
  bell: [
    'Clopoțel și fundal',
    'În meniu, Clopoțel îți permite să alegi dintre trei clopoțele, de la cel mai înalt la cel mai grav. Dacă îi activezi butonul, pe ecranul Numărare apare un clopoțel lângă Start: a-l atinge e ca și cum l-ai lovi. Îl poți suna în timp ce reciți: contorul nu-l aude, pentru că, cât timp sună, îi scoate notele din microfon.\n\nÎn Setări → Fundal alege lotusul sau una dintre cele cinci ilustrații (Sutra Lotusului, Buddha, templu, natură, flori de cireș), fiecare luminoasă sau întunecată după temă, ori o imagine proprie.',
  ],
  backup: [
    'Copie de rezervă și schimbarea telefonului',
    'În Setări, la „Datele tale”, creează o copie de rezervă: conține sesiunile, obiectivul, setările și înregistrările vocii tale. Păstreaz-o în afara telefonului (Drive, e-mail), iar pe telefonul nou folosește „Restaurează dintr-o copie de rezervă”. Poți și exporta sesiunile într-un fișier CSV.',
  ],
  menu: [
    'Meniul',
    'Atinge ☰ din stânga sus sau glisează spre dreapta pornind din jumătatea stângă a ecranului pentru a deschide meniul: contor, Gongyo, repere, mementouri, clopoțel, dicționarul conceptelor budiste, donații, acest ghid și confidențialitate. Pentru a-l închide glisează spre stânga sau atinge în afara lui.\n\nÎn Gongyo textul urcă precum genericul unui film, iar literele se umplu de culoare pe măsură ce le reciți, ca la karaoke: la mijloc cele două rânduri de citit, deasupra cele două deja recitate, dedesubt cele care urmează. Setează viteza cu cursorul: cât îl miști vezi o previzualizare. „Traducere” conține ambele capitole rând cu rând, în limba pe care o alegi.\n\nDe fiecare dată când deschizi aplicația te așteaptă citatul zilei, din scrierile lui Nichiren Daishonin, din Sutra Lotusului sau de la președinții Soka Gakkai. Săgețile (sau o glisare) arată citatele altor zile; „Intră” te duce în aplicație.',
  ],
  trouble: [
    'Dacă ceva nu merge',
    '• „Aplicația nu poate folosi microfonul”: permite microfonul în setările Android (Aplicații → Daimoku Counter → Permisiuni).\n• Nicio notificare cu ecranul stins: permite notificările aplicației.\n• Numărarea se oprește cu ecranul stins: unele telefoane închid aplicațiile pentru a economisi bateria; exclude Daimoku Counter de la optimizarea bateriei.\n• Numărarea greșește: după Stop atinge „Salvează audio pentru a raporta o problemă” și trimite fișierul dezvoltatorului (contactul este în politica de confidențialitate completă, pe pagina Confidențialitate).',
  ],
} satisfies HelpTexts;
