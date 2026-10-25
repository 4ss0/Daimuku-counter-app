# Languages

Every folder here is a language of the app, named with its code (`it`, `pt`,
`zh`…). Adding a folder adds the language: it shows up in Settings and is
chosen automatically on devices set to it.

| File | What |
|---|---|
| `meta.ts` | The language's own name, its English name, the tag for dates and numbers |
| `ui.ts` | The texts of the screens (keys as in `en/ui.ts`, `{placeholders}` kept) |
| `help.ts` | The "How to use" guide (a blank line starts a paragraph, `• ` a list item) |
| `dictionary.ts` | The dictionary: headword, second line, explanation of each entry |
| `quotes.ts` | The 366 quotes of the day, in the order of `../quotes/pools.ts` |
| `gongyo.ts` | The translation of Gongyo, line by line as in `../gongyoTranslation.ts` |
| `sources.ts` | Where the quotes come from: names of the writings, authors, sutras |
| `index.ts` | Puts the files together (the same in every language) |

Plurals: besides `{n}`, a text can choose a word by the language's plural
rules, as `{n:one=год|few=года|many=лет|other=года}` (the categories of
`Intl.PluralRules`; `other` is the fallback). English needs it nowhere, but
Russian, Polish, Czech and other languages do: «{n} {n:one=день|few=дня|many=дней|other=дня}».

English (`en`) is the reference: TypeScript checks that every language has
the same keys and as many quotes and lines, and English stands in for
anything missing at run time. Only English is in the app's main bundle; the
other languages are loaded when chosen.

After changing a language:

    npm run i18n:check      # placeholders, guide lists, nothing empty
    npm run i18n:android    # the Android texts (notification channels)
    npm run check
