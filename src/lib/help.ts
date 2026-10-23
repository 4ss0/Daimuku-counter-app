// "How to use" page: one section per topic, in three languages.
// In a body, a blank line starts a new paragraph and a line starting with
// "• " is a list item.
import type { Lang } from './i18n';

export interface HelpSection {
  id: string;
  icon: 'voice' | 'count' | 'coin' | 'moon' | 'tips' | 'stats' | 'goal' | 'badge' | 'bell' | 'backup' | 'menu' | 'help';
  text: Record<Lang, [title: string, body: string]>;
}

export const HELP: HelpSection[] = [
  {
    id: 'voice',
    icon: 'voice',
    text: {
      it: [
        'Prima di iniziare: la tua voce',
        'Al primo avvio l’app apre «La tua voce». Scegli una velocità (Lento, Medio o Veloce), imposta quanti daimoku reciterai, premi Registra, recita esattamente quel numero di daimoku e premi Fine. Poi tocca Aggiungi: l’app impara da quella registrazione.\n\nFai almeno una registrazione per ciascuna velocità. Puoi aggiungerne altre quando vuoi: se il conteggio sbaglia spesso, una registrazione in più alla velocità a cui reciti di solito aiuta molto.',
      ],
      en: [
        'Before you start: your voice',
        'On first start the app opens "Your voice". Pick a speed (Slow, Medium or Fast), set how many daimoku you will chant, press Record, chant exactly that many daimoku and press Done. Then tap Add: the app learns from that recording.\n\nMake at least one recording at each speed. You can add more at any time: if counting is often wrong, one more recording at the speed you usually chant helps a lot.',
      ],
      ja: [
        'はじめに：声の登録',
        '初回起動時は「声の登録」画面が開きます。速さ（ゆっくり・ふつう・速く）を選び、唱える遍数を設定して「録音」を押し、その遍数ちょうどの題目を唱えて「終了」を押します。次に「追加」をタップすると、アプリがその録音から学習します。\n\nそれぞれの速さで少なくとも1回ずつ録音してください。録音はいつでも追加できます。カウントがよく外れる場合は、普段唱える速さでもう1回録音すると大きく改善します。',
      ],
    },
  },
  {
    id: 'count',
    icon: 'count',
    text: {
      it: [
        'Contare i daimoku',
        'Nella schermata Conta premi Inizia e recita. Il numero sale appena l’app sente il «kyo» finale di ogni daimoku; il punto accanto allo stato si accende quando ti sente.\n\nQuando hai finito premi Ferma: la sessione viene salvata. Se l’app ne ha persi o aggiunti, correggi il numero con − e +: le statistiche usano il valore corretto. Puoi anche eliminare la sessione.',
      ],
      en: [
        'Counting daimoku',
        'On the Count screen press Start and chant. The number goes up as soon as the app hears the final "kyo" of each daimoku; the dot next to the status lights up when it hears you.\n\nWhen you have finished press Stop: the session is saved. If the app missed some or added too many, correct the number with − and +: the statistics use the corrected value. You can also delete the session.',
      ],
      ja: [
        '題目を数える',
        '「唱題」画面で「開始」を押して唱えます。各題目の最後の「経」が聞こえた時点で数が増えます。状態表示の横の点は、声が聞こえている間点灯します。\n\n終わったら「停止」を押すと、セッションが保存されます。数え漏れや数えすぎがあれば − と + で修正してください。記録には修正後の数が使われます。セッションを削除することもできます。',
      ],
    },
  },
  {
    id: 'coins',
    icon: 'coin',
    text: {
      it: [
        'Le monete: i daimoku ritrovati',
        'Oltre al riconoscimento immediato, ogni due secondi l’app riascolta l’ultimo minuto con più attenzione. Se trova daimoku che erano sfuggiti (coperti da una campana, dal juzu o da un rumore), li aggiunge: li vedi arrivare come monete che volano nel contatore.',
      ],
      en: [
        'Coins: recovered daimoku',
        'Besides the immediate recognition, every two seconds the app listens again, more carefully, to the last minute. If it finds daimoku that slipped through (covered by a bell, the beads or a noise), it adds them: you see them arrive as coins flying into the counter.',
      ],
      ja: [
        'コイン：見つけ直した題目',
        'その場での認識に加えて、アプリは2秒ごとに直前の1分間をより丁寧に聞き直します。鈴や数珠、雑音にまぎれて数え漏れた題目が見つかると、コインとしてカウンターに飛んでいき、加算されます。',
      ],
    },
  },
  {
    id: 'screen',
    icon: 'moon',
    text: {
      it: [
        'Con lo schermo spento',
        'Puoi spegnere lo schermo o usare altre app: il conteggio continua e una notifica mostra il numero aggiornato. Quando torni nell’app il contatore è già al numero giusto.\n\nIn Impostazioni puoi scegliere di tenere lo schermo acceso durante il conteggio. Anche se passi a un’altra schermata (Statistiche, Dizionario…) la sessione continua: nella barra in alto vedi il conteggio, toccalo per tornare al contatore.',
      ],
      en: [
        'With the screen off',
        'You can turn the screen off or use other apps: counting goes on and a notification shows the current number. When you come back to the app the counter already shows the right number.\n\nIn Settings you can choose to keep the screen on while counting. Even if you switch to another screen (Statistics, Dictionary…) the session goes on: the top bar shows the count, tap it to go back to the counter.',
      ],
      ja: [
        '画面を消しているとき',
        '画面を消したり他のアプリを使ったりしても、カウントは続き、通知に現在の数が表示されます。アプリに戻ると、カウンターはすでに正しい数になっています。\n\n「設定」で、カウント中に画面をつけたままにすることもできます。別の画面（記録、用語集など）に移ってもセッションは続き、上部のバーに数が表示されます。タップするとカウンターに戻ります。',
      ],
    },
  },
  {
    id: 'tips',
    icon: 'tips',
    text: {
      it: [
        'Consigli per un conteggio preciso',
        '• Tieni il telefono vicino a te, senza coprire il microfono.\n• Pronuncia ogni daimoku per intero, fino al «kyo»: una frase interrotta prima non viene contata.\n• Rumori forti e continui vicino al telefono (la campana, il juzu strofinato) possono far perdere qualche daimoku: i controlli ne recuperano la maggior parte.\n• In gruppo il conteggio funziona, ma è più preciso quando la voce più vicina al telefono è la tua.\n• Se il conteggio sbaglia spesso, aggiungi registrazioni della tua voce.',
      ],
      en: [
        'Tips for accurate counting',
        '• Keep the phone close to you, without covering the microphone.\n• Say each daimoku in full, up to "kyo": a phrase broken off before it is not counted.\n• Loud, steady noises near the phone (the bell, beads rubbed together) can make a few daimoku slip through: the checks recover most of them.\n• Counting works in a group, but it is more accurate when the voice closest to the phone is yours.\n• If counting is often wrong, add recordings of your voice.',
      ],
      ja: [
        '正確に数えるためのコツ',
        '• 電話は近くに置き、マイクをふさがないようにしてください。\n• 題目は「経」まで最後まで唱えてください。途中で切れた題目は数えられません。\n• 電話の近くでの大きく続く音（鈴、数珠をこする音など）で数え漏れが出ることがありますが、ほとんどは後から見つけ直されます。\n• 大勢での唱題でも数えられますが、電話に一番近い声がご自身の声だとより正確です。\n• よく外れる場合は、声の録音を追加してください。',
      ],
    },
  },
  {
    id: 'stats',
    icon: 'stats',
    text: {
      it: [
        'Statistiche, obiettivo e condivisione',
        'In Statistiche vedi i daimoku per giorno, settimana, mese e anno, i giorni di fila e la media: tocca una barra per il dettaglio. Imposta l’obiettivo giornaliero: l’anello della schermata Conta si riempie fino a raggiungerlo.\n\nHai recitato senza l’app, per esempio a una riunione? Sotto «Sessioni» scegli il giorno, la sessione (mattina o sera) e scrivi quanti Daimoku hai recitato. Con l’icona di condivisione puoi condividere una sessione, un periodo o l’obiettivo raggiunto.',
      ],
      en: [
        'Statistics, goal and sharing',
        'In Statistics you see your daimoku by day, week, month and year, your streak and your average: tap a bar for details. Set a daily goal: the ring on the Count screen fills up until you reach it.\n\nDid you chant without the app, for example at a meeting? Under "Sessions" choose the day and the session (morning or evening) and type how many Daimoku you chanted. With the share icon you can share a session, a period or a goal reached.',
      ],
      ja: [
        '記録・目標・共有',
        '「記録」では、日・週・月・年ごとの遍数、連続日数、平均を確認できます。棒をタップすると詳細が表示されます。毎日の目標を設定すると、「唱題」画面のリングが目標に向かって満ちていきます。\n\n会合などアプリを使わずに唱えたときは、「セッション」の下で日付と朝・夜を選び、唱えた遍数を入力してください。共有アイコンから、セッション、期間の記録、目標達成を共有できます。',
      ],
    },
  },
  {
    id: 'goals',
    icon: 'goal',
    text: {
      it: [
        'Obiettivi e promemoria',
        'In Impostazioni → Obiettivi scegli l’obiettivo giornaliero e, se vuoi, uno settimanale, mensile e annuale. L’obiettivo del giorno è diviso in due sessioni: la mattina (fino alle 14:00) e la sera; nella schermata Conta vedi a che punto sei con ciascuna.\n\nIl contatore continua la sessione in corso, partendo dai Daimoku già fatti. Per continuare l’altra (per esempio la recitazione del mattino fatta nel pomeriggio), toccala prima di premere Inizia. Ogni sessione salvata resta della mattina o della sera che hai scelto.\n\nNel menù, Promemoria ti manda una notifica all’ora che scegli per la sessione del mattino e per quella della sera.',
      ],
      en: [
        'Goals and reminders',
        'In Settings → Goals choose your daily goal and, if you like, a weekly, monthly and yearly one. The daily goal is split in two sessions: the morning (until 2 pm) and the evening; the Count screen shows how far you are with each.\n\nThe counter continues the current session, starting from the Daimoku already chanted. To continue the other one (say, the morning chanting done in the afternoon), tap it before pressing Start. Every saved session stays in the morning or the evening you chose.\n\nIn the menu, Reminders sends you a notification at the time you choose for the morning and for the evening session.',
      ],
      ja: [
        '目標とリマインダー',
        '「設定」→「目標」で毎日の目標と、必要なら毎週・毎月・毎年の目標を設定します。毎日の目標は朝（14時まで）と夜の2回に分かれ、「唱題」画面でそれぞれの進み具合が分かります。\n\nカウンターは今の時間帯のお勤めを、すでに唱えた数から続けます。もう一方を続けたいとき（たとえば朝の分を午後に唱えるとき）は、開始を押す前にそちらをタップしてください。保存したセッションは選んだ朝・夜のまま記録されます。\n\nメニューの「リマインダー」では、朝と夜それぞれ好きな時刻にお知らせを受け取れます。',
      ],
    },
  },
  {
    id: 'badges',
    icon: 'badge',
    text: {
      it: [
        'Traguardi e giorno abbuonato',
        'Ricevi un traguardo a 100, 1.000, 10.000, 1 milione, 5 e 10 milioni di daimoku, e per i giorni di recitazione consecutivi: una settimana, un mese, sei mesi, poi 1, 2, 5, 10, 15 e 20 anni. Li trovi tutti in Statistiche.\n\nSe un giorno non hai recitato, una volta a settimana puoi abbuonarlo con il tasto in Statistiche: la serie di giorni consecutivi non si interrompe.',
      ],
      en: [
        'Milestones and the forgiven day',
        'You earn a milestone at 100, 1,000, 10,000, 1 million, 5 and 10 million daimoku, and for days of chanting in a row: a week, a month, six months, then 1, 2, 5, 10, 15 and 20 years. You find them all in Statistics.\n\nIf you did not chant one day, once a week you can forgive it with the button in Statistics: your run of days in a row does not break.',
      ],
      ja: [
        '達成バッジとお休みの日',
        '通算100・1,000・10,000・100万・500万・1,000万遍と、連続日数（1週間、1か月、6か月、1・2・5・10・15・20年）でバッジがもらえます。すべて「記録」で確認できます。\n\n唱題できなかった日は、週に1日だけ「記録」のボタンで「お休み」にでき、連続日数が途切れません。',
      ],
    },
  },
  {
    id: 'bell',
    icon: 'bell',
    text: {
      it: [
        'Campana e sfondo',
        'Nel menù, Campana ti fa scegliere tra tre campane, dalla più acuta alla più profonda. Se attivi il tasto, nella schermata Conta compare una campana accanto a Inizia: toccarla è come colpirla. Puoi suonarla mentre reciti: il contatore non la sente, perché toglie dal microfono le note di quella campana finché suona.\n\nIn Impostazioni → Sfondo scegli tra il loto e cinque illustrazioni (Sutra del Loto, Budda, tempio, natura, ciliegi), ognuna chiara o scura secondo il tema, oppure una tua immagine.',
      ],
      en: [
        'Bell and background',
        'In the menu, Bell lets you choose among three bells, from the highest to the deepest. If you turn its button on, a bell appears next to Start on the Count screen: tapping it is like striking it. You can ring it while chanting: the counter does not hear it, as it takes that bell's notes out of the microphone while it rings.\n\nIn Settings → Background choose the lotus or one of five illustrations (Lotus Sutra, Buddha, temple, nature, cherry blossoms), each light or dark with the theme, or a picture of your own.',
      ],
      ja: [
        '鈴と背景',
        'メニューの「鈴（りん）」では、高い音から低い音まで3種類の鈴を選べます。ボタンをオンにすると「唱題」画面の「開始」の横に鈴が表示され、タップすると鈴を打つのと同じように鳴ります。唱題しながら鳴らしても大丈夫です。鳴っている間、その鈴の音をマイクから取り除くので、カウンターには聞こえません。\n\n「設定」→「背景」では、蓮と5つのイラスト（法華経・仏陀・寺院・自然・桜、テーマに合わせてライト／ダーク）、または自分の画像から選べます。',
      ],
    },
  },
  {
    id: 'backup',
    icon: 'backup',
    text: {
      it: [
        'Backup e cambio di telefono',
        'In Impostazioni, alla voce «I tuoi dati», crea un backup: contiene sessioni, obiettivo, impostazioni e le registrazioni della tua voce. Conservalo fuori dal telefono (Drive, e-mail) e sul nuovo telefono usa «Ripristina da un backup». Puoi anche esportare le sessioni in un file CSV.',
      ],
      en: [
        'Backup and changing phone',
        'In Settings, under "Your data", create a backup: it holds your sessions, goal, settings and voice recordings. Keep it off the phone (Drive, e-mail) and on the new phone use "Restore from a backup". You can also export your sessions to a CSV file.',
      ],
      ja: [
        'バックアップと機種変更',
        '「設定」の「データ」でバックアップを作成できます。記録、目標、設定、声の録音が含まれます。Drive やメールなど端末の外に保管し、新しい端末で「バックアップから復元」を使ってください。記録を CSV ファイルに書き出すこともできます。',
      ],
    },
  },
  {
    id: 'menu',
    icon: 'menu',
    text: {
      it: [
        'Il menù',
        'Tocca ☰ in alto a sinistra, oppure scorri verso destra partendo dalla metà sinistra dello schermo, per aprire il menù: contatore, promemoria, campana, dizionario dei concetti buddisti, donazioni, questa guida e privacy. Per chiuderlo scorri verso sinistra o tocca fuori.',
      ],
      en: [
        'The menu',
        'Tap ☰ at the top left, or swipe to the right starting from the left half of the screen, to open the menu: counter, reminders, bell, dictionary of Buddhist concepts, donations, this guide and privacy. To close it swipe to the left or tap outside.',
      ],
      ja: [
        'メニュー',
        '左上の ☰ をタップするか、画面の左半分から右へスワイプするとメニューが開きます。カウンター、リマインダー、鈴、仏法用語集、寄付、この使い方、プライバシーに移動できます。閉じるには左へスワイプするか、外側をタップします。',
      ],
    },
  },
  {
    id: 'trouble',
    icon: 'help',
    text: {
      it: [
        'Se qualcosa non va',
        '• «L’app non può usare il microfono»: consenti il microfono nelle impostazioni di Android (App → Daimoku Counter → Autorizzazioni).\n• Non vedi la notifica a schermo spento: consenti le notifiche dell’app.\n• A schermo spento il conteggio si ferma: alcuni telefoni chiudono le app per risparmiare batteria; togli Daimoku Counter dall’ottimizzazione della batteria.\n• Il conteggio sbaglia: dopo Ferma tocca «Salva l\'audio per segnalare un errore» e invia il file allo sviluppatore (il contatto è nell’informativa completa, nella pagina Privacy).',
      ],
      en: [
        'If something goes wrong',
        '• "The app can\'t use the microphone": allow the microphone in Android settings (Apps → Daimoku Counter → Permissions).\n• No notification with the screen off: allow the app’s notifications.\n• Counting stops with the screen off: some phones close apps to save battery; exclude Daimoku Counter from battery optimisation.\n• Counting is wrong: after Stop tap "Save the audio to report a problem" and send the file to the developer (the contact is in the full privacy policy, on the Privacy page).',
      ],
      ja: [
        'うまくいかないとき',
        '• 「マイクを使用できません」と表示される場合：Android の設定（アプリ → Daimoku Counter → 権限）でマイクを許可してください。\n• 画面オフ時に通知が出ない：アプリの通知を許可してください。\n• 画面オフ時にカウントが止まる：電池節約のためにアプリを終了する機種があります。Daimoku Counter を電池の最適化の対象から外してください。\n• カウントが外れる：停止後に「不具合報告用に音声を保存」をタップし、ファイルを開発者に送ってください（連絡先は「プライバシー」ページのポリシー全文にあります）。',
      ],
    },
  },
];

/** Paragraphs and lists of a section body. */
export function blocks(body: string): ({ p: string } | { list: string[] })[] {
  return body.split('\n\n').map((chunk) => {
    const lines = chunk.split('\n');
    return lines.every((l) => l.startsWith('• ')) ? { list: lines.map((l) => l.slice(2)) } : { p: chunk };
  });
}
