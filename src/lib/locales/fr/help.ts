import type { HelpTexts } from '../types';

export default {
  voice: [
    'Avant de commencer : votre voix',
    'Au premier lancement, l’appli ouvre « Votre voix ». Choisissez une vitesse (Lent, Moyen ou Rapide), indiquez combien de daimoku vous allez réciter, appuyez sur Enregistrer, récitez exactement ce nombre de daimoku et appuyez sur Terminer. Touchez ensuite Ajouter : l’appli apprend à partir de cet enregistrement.\n\nFaites au moins un enregistrement à chaque vitesse. Vous pouvez en ajouter d’autres à tout moment : si le comptage se trompe souvent, un enregistrement de plus à la vitesse où vous récitez d’habitude aide beaucoup.',
  ],
  count: [
    'Compter les daimoku',
    'Sur l’écran Compter, appuyez sur Démarrer et récitez. Le nombre augmente dès que l’appli entend le « kyo » final de chaque daimoku ; le point à côté de l’état s’allume quand elle vous entend.\n\nQuand vous avez fini, appuyez sur Arrêter : la séance est enregistrée. Si l’appli en a manqué ou en a ajouté trop, corrigez le nombre avec − et + : les statistiques utilisent la valeur corrigée. Vous pouvez aussi supprimer la séance.',
  ],
  coins: [
    'Les pièces : les daimoku récupérés',
    'En plus de la reconnaissance immédiate, toutes les deux secondes l’appli réécoute plus attentivement la dernière minute. Si elle trouve des daimoku qui lui ont échappé (couverts par la cloche, le juzu ou un bruit), elle les ajoute : vous les voyez arriver comme des pièces qui volent jusqu’au compteur.',
  ],
  screen: [
    'Écran éteint',
    'Vous pouvez éteindre l’écran ou utiliser d’autres applis : le comptage continue et une notification affiche le nombre actuel. Quand vous revenez dans l’appli, le compteur affiche déjà le bon nombre.\n\nDans Réglages, vous pouvez choisir de garder l’écran allumé pendant le comptage. Même si vous passez à un autre écran (Statistiques, Dictionnaire…), la séance continue : la barre du haut affiche le comptage, touchez-la pour revenir au compteur.',
  ],
  tips: [
    'Conseils pour un comptage précis',
    '• Gardez le téléphone près de vous, sans couvrir le micro.\n• Prononcez chaque daimoku en entier, jusqu’à « kyo » : une phrase interrompue avant n’est pas comptée.\n• Les bruits forts et continus près du téléphone (la cloche, le juzu frotté) peuvent faire échapper quelques daimoku : les vérifications en récupèrent la plupart.\n• Le comptage fonctionne en groupe, mais il est plus précis quand la voix la plus proche du téléphone est la vôtre.\n• Si le comptage se trompe souvent, ajoutez des enregistrements de votre voix.',
  ],
  stats: [
    'Statistiques, objectif et partage',
    'Dans Statistiques, vous voyez vos daimoku par jour, semaine, mois et année, votre série et votre moyenne : touchez un point du graphique pour le détail. Dans Réglages, choisissez quelle période s’affiche en premier. Fixez un objectif quotidien : l’anneau de l’écran Compter se remplit jusqu’à ce que vous l’atteigniez.\n\nVous avez récité sans l’appli, par exemple lors d’une réunion ? Sous « Séances », choisissez le jour et la séance (matin ou soir) et saisissez le nombre de daimoku récités. Avec l’icône de partage, vous pouvez partager une séance, une période ou un objectif atteint.',
  ],
  goals: [
    'Objectifs et rappels',
    'Dans Réglages → Objectifs de daimoku, choisissez votre objectif quotidien et, si vous le souhaitez, un objectif hebdomadaire, mensuel et annuel. L’objectif quotidien est réparti en deux séances : le matin (jusqu’à 14 h) et le soir ; l’écran Compter montre où vous en êtes pour chacune.\n\nLe compteur poursuit la séance en cours, à partir des daimoku déjà récités. Pour poursuivre l’autre (par exemple le daimoku du matin fait l’après-midi), touchez-la avant d’appuyer sur Démarrer. Chaque séance enregistrée reste dans le matin ou le soir que vous avez choisi.\n\nDans le menu, Rappels vous envoie une notification à l’heure choisie pour la séance du matin et pour celle du soir.',
  ],
  intents: [
    'Vos objectifs',
    'Dans le menu, Objectifs est l’endroit où écrire pour quoi vous récitez : un ou plusieurs objectifs, avec vos propres mots. Sous chacun, déplacez le curseur pour indiquer où vous en êtes.\n\nLe moment venu, touchez Fermer et choisissez pourquoi : l’objectif est atteint, ou vous avez compris qu’il existait un meilleur chemin. Les objectifs fermés restent en bas de la page : vous pouvez les relire, les rouvrir ou les supprimer.',
  ],
  badges: [
    'Étapes et jour pardonné',
    'Vous franchissez une étape à 100, 1 000, 10 000, 1 million, 5 et 10 millions de daimoku, et pour les jours de récitation d’affilée : une semaine, un mois, six mois, puis 1, 2, 5, 10, 15 et 20 ans. Vous les trouvez toutes dans le menu, sous Étapes, avec le jour pardonné.\n\nSi vous n’avez pas récité un jour, une fois par semaine vous pouvez le pardonner avec le bouton de Statistiques : votre série de jours d’affilée ne s’interrompt pas.',
  ],
  bell: [
    'Cloche et fond',
    'Dans le menu, Cloche vous laisse choisir entre trois cloches, de la plus aiguë à la plus grave. Si vous activez son bouton, une cloche apparaît à côté de Démarrer sur l’écran Compter : la toucher revient à la frapper. Vous pouvez la faire sonner pendant la récitation : le compteur ne l’entend pas, car il retire les notes de cette cloche du micro pendant qu’elle sonne.\n\nDans Réglages → Fond, choisissez le lotus ou l’une des cinq illustrations (Sûtra du Lotus, Bouddha, temple, nature, cerisiers en fleurs), chacune claire ou sombre selon le thème, ou une image à vous.',
  ],
  backup: [
    'Sauvegarde et changement de téléphone',
    'Dans Réglages, sous « Vos données », activez la Sauvegarde automatique : la première fois, vous choisissez où garder le fichier (sur le téléphone ou sur Drive), puis l’appli enregistre tout toute seule après chaque séance, chaque modification de vos enregistrements de voix et chaque étape franchie. Sur un nouveau téléphone, ou si vous réinstallez l’appli, choisissez « Charger une sauvegarde » au premier démarrage et sélectionnez ce fichier.\n\nDans Réglages, sous « Vos données », créez une sauvegarde : elle contient vos séances, objectifs, réglages et enregistrements de voix. Conservez-la hors du téléphone (Drive, e-mail) et, sur le nouveau téléphone, utilisez « Restaurer une sauvegarde ». Vous pouvez aussi exporter vos séances dans un fichier CSV.',
  ],
  menu: [
    'Le menu',
    'Touchez ☰ en haut à gauche, ou glissez vers la droite depuis la moitié gauche de l’écran, pour ouvrir le menu : compteur, objectifs, Gongyo, étapes, rappels, cloche, dictionnaire des notions bouddhiques, dons, ce mode d’emploi et confidentialité. Pour le fermer, glissez vers la gauche ou touchez à l’extérieur.\n\nDans Gongyo, le texte défile comme un générique de film et les lettres se colorent à mesure que vous récitez, comme au karaoké : au milieu les deux lignes à lire, au-dessus les deux déjà récitées, en dessous les suivantes. Réglez la vitesse avec le curseur : pendant que vous le déplacez, vous voyez un aperçu. Dans « Traduction », vous trouvez les deux chapitres ligne par ligne, dans la langue de votre choix.\n\nÀ chaque ouverture de l’appli, vous trouvez la citation du jour, tirée des écrits de Nichiren Daishonin, du Sûtra du Lotus ou des présidents de la Soka Gakkai. Les flèches (ou un glissement) montrent les citations des autres jours ; « Entrer » vous mène dans l’appli.',
  ],
  trouble: [
    'En cas de problème',
    '• « L’appli ne peut pas utiliser le micro » : autorisez le micro dans les réglages d’Android (Applications → Daimoku Counter → Autorisations).\n• Pas de notification écran éteint : autorisez les notifications de l’appli.\n• Le comptage s’arrête écran éteint : certains téléphones ferment les applis pour économiser la batterie ; excluez Daimoku Counter de l’optimisation de la batterie.\n• Le comptage se trompe : après Arrêter, touchez « Enregistrer l’audio pour signaler un problème » et envoyez le fichier au développeur (le contact figure dans la politique de confidentialité complète, sur la page Confidentialité).',
  ],
} satisfies HelpTexts;
