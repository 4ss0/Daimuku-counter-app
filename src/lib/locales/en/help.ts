import type { HelpTexts } from '../types';

export default {
  voice: [
    'Before you start: your voice',
    'On first start the app opens "Your voice". Pick a speed (Slow, Medium or Fast), set how many daimoku you will chant, press Record, chant exactly that many daimoku and press Done. Then tap Add: the app learns from that recording.\n\nMake at least one recording at each speed. You can add more at any time: if counting is often wrong, one more recording at the speed you usually chant helps a lot.',
  ],
  count: [
    'Counting daimoku',
    'On the Count screen press Start and chant. The number goes up as soon as the app hears the final "kyo" of each daimoku; the dot next to the status lights up when it hears you.\n\nWhen you have finished press Stop: the session is saved. If the app missed some or added too many, correct the number with − and +: the statistics use the corrected value. You can also delete the session.',
  ],
  coins: [
    'Coins: recovered daimoku',
    'Besides the immediate recognition, every two seconds the app listens again, more carefully, to the last minute. If it finds daimoku that slipped through (covered by a bell, the beads or a noise), it adds them: you see them arrive as coins flying into the counter.',
  ],
  screen: [
    'With the screen off',
    'You can turn the screen off or use other apps: counting goes on and a notification shows the current number. When you come back to the app the counter already shows the right number.\n\nIn Settings you can choose to keep the screen on while counting. Even if you switch to another screen (Statistics, Dictionary…) the session goes on: the top bar shows the count, tap it to go back to the counter.',
  ],
  tips: [
    'Tips for accurate counting',
    '• Keep the phone close to you, without covering the microphone.\n• Say each daimoku in full, up to "kyo": a phrase broken off before it is not counted.\n• Loud, steady noises near the phone (the bell, beads rubbed together) can make a few daimoku slip through: the checks recover most of them.\n• Counting works in a group, but it is more accurate when the voice closest to the phone is yours.\n• If counting is often wrong, add recordings of your voice.',
  ],
  stats: [
    'Statistics, goal and sharing',
    'In Statistics you see your daimoku by day, week, month and year, your streak and your average: tap a point of the chart for details. In Settings choose which period is shown first. Set a daily goal: the ring on the Count screen fills up until you reach it.\n\nDid you chant without the app, for example at a meeting? Under "Sessions" choose the day and the session (morning or evening) and type how many Daimoku you chanted. With the share icon you can share a session, a period or a goal reached.',
  ],
  goals: [
    'Goals and reminders',
    'In Settings → Goals choose your daily goal and, if you like, a weekly, monthly and yearly one. The daily goal is split in two sessions: the morning (until 2 pm) and the evening; the Count screen shows how far you are with each.\n\nThe counter continues the current session, starting from the Daimoku already chanted. To continue the other one (say, the morning chanting done in the afternoon), tap it before pressing Start. Every saved session stays in the morning or the evening you chose.\n\nIn the menu, Reminders sends you a notification at the time you choose for the morning and for the evening session.',
  ],
  badges: [
    'Milestones and the forgiven day',
    'You earn a milestone at 100, 1,000, 10,000, 1 million, 5 and 10 million daimoku, and for days of chanting in a row: a week, a month, six months, then 1, 2, 5, 10, 15 and 20 years. You find them all in the menu, under Milestones, together with the forgiven day.\n\nIf you did not chant one day, once a week you can forgive it with the button in Statistics: your run of days in a row does not break.',
  ],
  bell: [
    'Bell and background',
    'In the menu, Bell lets you choose among three bells, from the highest to the deepest. If you turn its button on, a bell appears next to Start on the Count screen: tapping it is like striking it. You can ring it while chanting: the counter does not hear it, as it takes that bell’s notes out of the microphone while it rings.\n\nIn Settings → Background choose the lotus or one of five illustrations (Lotus Sutra, Buddha, temple, nature, cherry blossoms), each light or dark with the theme, or a picture of your own.',
  ],
  backup: [
    'Backup and changing phone',
    'In Settings, under "Your data", create a backup: it holds your sessions, goal, settings and voice recordings. Keep it off the phone (Drive, e-mail) and on the new phone use "Restore from a backup". You can also export your sessions to a CSV file.',
  ],
  menu: [
    'The menu',
    'Tap ☰ at the top left, or swipe to the right starting from the left half of the screen, to open the menu: counter, Gongyo, milestones, reminders, bell, dictionary of Buddhist concepts, donations, this guide and privacy. To close it swipe to the left or tap outside.\n\nIn Gongyo the text rolls up like film credits and the letters fill with colour as you recite them, as in karaoke: in the middle the two lines to read, above the two already recited, below the ones that follow. Set the speed with the slider: while you move it you see a preview. “Translation” has both chapters line by line, in the language you choose.\n\nEach time you open the app you find the quote of the day, from the writings of Nichiren Daishonin, the Lotus Sutra or the presidents of the Soka Gakkai. The arrows (or a swipe) show the other days’ quotes; “Enter” takes you into the app.',
  ],
  trouble: [
    'If something goes wrong',
    '• "The app can\'t use the microphone": allow the microphone in Android settings (Apps → Daimoku Counter → Permissions).\n• No notification with the screen off: allow the app’s notifications.\n• Counting stops with the screen off: some phones close apps to save battery; exclude Daimoku Counter from battery optimisation.\n• Counting is wrong: after Stop tap "Save the audio to report a problem" and send the file to the developer (the contact is in the full privacy policy, on the Privacy page).',
  ],
} satisfies HelpTexts;
