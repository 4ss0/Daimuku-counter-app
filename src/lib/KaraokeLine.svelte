<script lang="ts">
  // One line of Gongyo as karaoke: the letters of the readings already
  // recited are in the accent colour, and the reading being recited fills
  // with it from the left. A line too long for the width wraps only between
  // its phrases (after a full stop), never inside one.
  import type { Piece } from './gongyo';

  export let pieces: Piece[];
  /** Beats recited so far in this line (0 = not started). */
  export let beat = 0;

  // the beat of each piece (-1 for spaces and stops)
  $: beatOf = pieces.reduce<number[]>((a, p) => {
    const n = a.filter((x) => x >= 0).length;
    a.push(p.beat ? n : -1);
    return a;
  }, []);
  /** How much of a piece is recited, 0..1 (spaces follow the reading before them). */
  function fill(i: number, b: number): number {
    const k = beatOf[i];
    if (k >= 0) return Math.min(1, Math.max(0, b - k));
    let j = i - 1;
    while (j >= 0 && beatOf[j] < 0) j--;
    return j >= 0 && b >= beatOf[j] + 1 ? 1 : 0;
  }

  // the phrases: the pieces up to a full stop followed by a space
  $: phrases = pieces.reduce<number[][]>(
    (a, p, i) => {
      a[a.length - 1].push(i);
      if (!p.beat && /\.\s+$/.test(p.text) && i < pieces.length - 1) a.push([]);
      return a;
    },
    [[]],
  );
  const shown = (p: Piece, last: boolean) => (last ? p.text.trimEnd() : p.text);
</script>

<!-- no whitespace between the pieces: they are parts of one word -->
<span class="kl"
  >{#each phrases as ph, n}{#if n > 0}{' '}{/if}<span class="ph"
      >{#each ph as i, k}{@const p = pieces[i]}{@const f = fill(i, beat)}{#if p.beat}<span
            class:done={f >= 1}
            class:wipe={f > 0 && f < 1}
            style={f > 0 && f < 1 ? `--f:${(f * 100).toFixed(1)}%` : ''}>{p.text}</span
          >{:else}<span class:done={f >= 1}>{shown(p, k === ph.length - 1 && n < phrases.length - 1)}</span>{/if}{/each}</span
    >{/each}</span
>

<style>
  .kl {
    white-space: normal;
  }
  .ph {
    white-space: nowrap;
  }
  .done {
    color: var(--accent);
  }
  /* the reading being recited: filled with the accent up to --f */
  .wipe {
    color: transparent;
    background-image: linear-gradient(90deg, var(--accent) var(--f), var(--k-text, var(--text)) var(--f));
    -webkit-background-clip: text;
    background-clip: text;
  }
</style>
