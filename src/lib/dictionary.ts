// Dictionary of the main concepts of Nichiren Daishonin's Buddhism: the
// entries, with the Japanese term and its reading. Their headword, second
// line and explanation, in each language, are in
// src/lib/locales/<code>/dictionary.ts.
import type { Locale } from './locales/types';

export type Cat = 'practice' | 'doctrine' | 'history';

export const CATS: Cat[] = ['practice', 'doctrine', 'history'];

export const TERMS = [
  { id: 'daimoku', cat: 'practice', jp: '題目', kana: 'だいもく', see: ['nmrk', 'gongyo', 'gohonzon', 'sandai-hiho'] },
  { id: 'nmrk', cat: 'practice', jp: '南無妙法蓮華経', kana: 'なんみょうほうれんげきょう', see: ['daimoku', 'inga-guji', 'nichiren', 'lotus-sutra'] },
  { id: 'gongyo', cat: 'practice', jp: '勤行', kana: 'ごんぎょう', see: ['daimoku', 'gohonzon', 'lotus-sutra'] },
  { id: 'gohonzon', cat: 'practice', jp: '御本尊', kana: 'ごほんぞん', see: ['jikkai', 'butsudan', 'sandai-hiho', 'kyochi-myogo'] },
  { id: 'butsudan', cat: 'practice', jp: '仏壇', kana: 'ぶつだん', see: ['gohonzon'] },
  { id: 'juzu', cat: 'practice', jp: '数珠', kana: 'じゅず', see: ['gongyo', 'bonno-soku-bodai'] },
  { id: 'shinjin', cat: 'practice', jp: '信心', kana: 'しんじん', see: ['shin-gyo-gaku', 'shiriki'] },
  { id: 'shin-gyo-gaku', cat: 'practice', jp: '信・行・学', kana: 'しん・ぎょう・がく', see: ['shinjin', 'gosho', 'shakubuku'] },
  { id: 'shakubuku', cat: 'practice', jp: '折伏', kana: 'しゃくぶく', see: ['shoju', 'kosen-rufu', 'mappo', 'fukyo'] },
  { id: 'shoju', cat: 'practice', jp: '摂受', kana: 'しょうじゅ', see: ['shakubuku'] },
  { id: 'kosen-rufu', cat: 'practice', jp: '広宣流布', kana: 'こうせんるふ', see: ['shakubuku', 'jiyu-bosatsu', 'rissho-ankoku'] },
  { id: 'zadankai', cat: 'practice', jp: '座談会', kana: 'ざだんかい', see: ['soka-gakkai', 'itai-doshin'] },
  { id: 'ichinen', cat: 'practice', jp: '一念', kana: 'いちねん', see: ['ichinen-sanzen', 'esho-funi', 'jikkai'] },
  { id: 'jikkai', cat: 'doctrine', jp: '十界', kana: 'じっかい', see: ['jikkai-gogu', 'ichinen-sanzen', 'buddhahood'] },
  { id: 'jikkai-gogu', cat: 'doctrine', jp: '十界互具', kana: 'じっかいごぐ', see: ['jikkai', 'ichinen-sanzen', 'sokushin-jobutsu'] },
  { id: 'junyoze', cat: 'doctrine', jp: '十如是', kana: 'じゅうにょぜ', see: ['ichinen-sanzen', 'gongyo'] },
  { id: 'san-seken', cat: 'doctrine', jp: '三世間', kana: 'さんせけん', see: ['ichinen-sanzen', 'esho-funi'] },
  { id: 'ichinen-sanzen', cat: 'doctrine', jp: '一念三千', kana: 'いちねんさんぜん', see: ['ichinen', 'jikkai-gogu', 'junyoze', 'san-seken', 'tiantai'] },
  { id: 'buddhahood', cat: 'doctrine', jp: '仏界', kana: 'ぶっかい', see: ['jikkai', 'sokushin-jobutsu', 'gohonzon'] },
  { id: 'sokushin-jobutsu', cat: 'doctrine', jp: '即身成仏', kana: 'そくしんじょうぶつ', see: ['buddhahood', 'jikkai-gogu', 'bonno-soku-bodai'] },
  { id: 'ningen-kakumei', cat: 'doctrine', jp: '人間革命', kana: 'にんげんかくめい', see: ['shukumei-tenkan', 'soka-gakkai', 'ichinen'] },
  { id: 'karma', cat: 'doctrine', jp: '業', kana: 'ごう', see: ['shukumei-tenkan', 'inga-guji'] },
  { id: 'shukumei-tenkan', cat: 'doctrine', jp: '宿命転換', kana: 'しゅくめいてんかん', see: ['karma', 'ningen-kakumei'] },
  { id: 'esho-funi', cat: 'doctrine', jp: '依正不二', kana: 'えしょうふに', see: ['shiki-shin-funi', 'ichinen-sanzen', 'san-seken'] },
  { id: 'shiki-shin-funi', cat: 'doctrine', jp: '色心不二', kana: 'しきしんふに', see: ['esho-funi'] },
  { id: 'bonno-soku-bodai', cat: 'doctrine', jp: '煩悩即菩提', kana: 'ぼんのうそくぼだい', see: ['shoji-soku-nehan', 'sokushin-jobutsu'] },
  { id: 'shoji-soku-nehan', cat: 'doctrine', jp: '生死即涅槃', kana: 'しょうじそくねはん', see: ['bonno-soku-bodai'] },
  { id: 'inga-guji', cat: 'doctrine', jp: '因果倶時', kana: 'いんがぐじ', see: ['nmrk', 'karma'] },
  { id: 'santai', cat: 'doctrine', jp: '三諦', kana: 'さんたい', see: ['ichinen-sanzen', 'tiantai'] },
  { id: 'kushiki', cat: 'doctrine', jp: '九識', kana: 'くしき', see: ['karma', 'buddhahood'] },
  { id: 'sansho-shima', cat: 'doctrine', jp: '三障四魔', kana: 'さんしょうしま', see: ['shiriki', 'shoten-zenjin'] },
  { id: 'shiriki', cat: 'doctrine', jp: '四力', kana: 'しりき', see: ['shinjin', 'gohonzon', 'kudoku'] },
  { id: 'kudoku', cat: 'doctrine', jp: '功徳', kana: 'くどく', see: ['shiriki', 'kyochi-myogo'] },
  { id: 'kyochi-myogo', cat: 'doctrine', jp: '境智冥合', kana: 'きょうちみょうごう', see: ['gohonzon', 'daimoku'] },
  { id: 'itai-doshin', cat: 'doctrine', jp: '異体同心', kana: 'いたいどうしん', see: ['zadankai', 'kosen-rufu'] },
  { id: 'shitei-funi', cat: 'doctrine', jp: '師弟不二', kana: 'していふに', see: ['soka-gakkai', 'ningen-kakumei'] },
  { id: 'jiyu-bosatsu', cat: 'doctrine', jp: '地涌の菩薩', kana: 'じゆのぼさつ', see: ['kosen-rufu', 'lotus-sutra'] },
  { id: 'fukyo', cat: 'doctrine', jp: '不軽菩薩', kana: 'ふきょうぼさつ', see: ['shakubuku', 'jikkai-gogu'] },
  { id: 'shoten-zenjin', cat: 'doctrine', jp: '諸天善神', kana: 'しょてんぜんじん', see: ['sansho-shima', 'esho-funi'] },
  { id: 'sandai-hiho', cat: 'doctrine', jp: '三大秘法', kana: 'さんだいひほう', see: ['gohonzon', 'daimoku'] },
  { id: 'sansho', cat: 'doctrine', jp: '三証', kana: 'さんしょう', see: ['kudoku'] },
  { id: 'mappo', cat: 'doctrine', jp: '末法', kana: 'まっぽう', see: ['nichiren', 'shakubuku'] },
  { id: 'lotus-sutra', cat: 'history', jp: '法華経', kana: 'ほけきょう', see: ['nmrk', 'gongyo', 'shakyamuni', 'tiantai'] },
  { id: 'gosho', cat: 'history', jp: '御書', kana: 'ごしょ', see: ['nichiren', 'shin-gyo-gaku', 'rissho-ankoku'] },
  { id: 'rissho-ankoku', cat: 'history', jp: '立正安国論', kana: 'りっしょうあんこくろん', see: ['nichiren', 'kosen-rufu', 'gosho'] },
  { id: 'shakyamuni', cat: 'history', jp: '釈尊', kana: 'しゃくそん', see: ['lotus-sutra', 'mappo'] },
  { id: 'tiantai', cat: 'history', jp: '天台大師', kana: 'てんだいだいし', see: ['ichinen-sanzen', 'lotus-sutra', 'santai'] },
  { id: 'nichiren', cat: 'history', jp: '日蓮大聖人', kana: 'にちれんだいしょうにん', see: ['nmrk', 'gosho', 'rissho-ankoku', 'nikko'] },
  { id: 'nikko', cat: 'history', jp: '日興上人', kana: 'にっこうしょうにん', see: ['nichiren', 'gosho'] },
  { id: 'soka-gakkai', cat: 'history', jp: '創価学会', kana: 'そうかがっかい', see: ['zadankai', 'ningen-kakumei', 'shitei-funi'] },
] as const satisfies readonly { id: string; cat: Cat; jp: string; kana: string; see?: readonly string[] }[];

export type Term = (typeof TERMS)[number];
export type TermId = Term['id'];

/**
 * Headword, second line, the Japanese term to show next to the headword
 * ('' when the headword is that term already) and text of an entry.
 */
export function view(e: Term, texts: Locale): { title: string; sub: string; jp: string; text: string } {
  const [title, sub, text] = texts.dictionary[e.id];
  return { title, sub, jp: title === e.jp ? '' : e.jp, text };
}

/** Everything a search may match: the language shown, English and the Japanese term. */
export function haystack(e: Term, texts: Locale, english: Locale): string {
  return [e.jp, e.kana, ...texts.dictionary[e.id], english.dictionary[e.id][0]].join('\n').toLowerCase();
}
