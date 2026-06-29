<script lang="ts">
  import type { Decision, Photo } from '$lib/types';

  interface Props {
    photo: Photo;
    selected?: boolean;
    onclick?: (photo: Photo) => void;
  }

  let { photo, selected = false, onclick }: Props = $props();

  const decisionLabel = (decision: Decision) => {
    if (decision === 'keep') return 'K';
    if (decision === 'cull') return 'X';
    if (decision === 'auto') return 'A';
    return '';
  };

  const scoreClass = $derived(photo.score < 45 ? 'low' : photo.score < 65 ? 'mid' : '');
  const decisionClass = $derived(photo.decision === 'auto' ? 'auto' : photo.decision ?? '');
</script>

<button
  type="button"
  class={['photo-tile', selected && 'selected', (photo.decision === 'cull' || photo.decision === 'auto') && 'culled']
    .filter(Boolean)
    .join(' ')}
  aria-label={`选择 ${photo.name}`}
  onclick={() => onclick?.(photo)}
>
  {#if photo.sourceUrl}
    <img class="photo-fill image" src={photo.sourceUrl} alt="" loading="lazy" />
  {:else}
    <span class="photo-fill" style:background={photo.palette}></span>
  {/if}
  <span class={['score-badge', scoreClass].filter(Boolean).join(' ')}>{photo.score}</span>
  {#if photo.decision}
    <span class={['decision-mark', decisionClass].filter(Boolean).join(' ')}>{decisionLabel(photo.decision)}</span>
  {/if}
  <span class="photo-info">
    <span class="seq">{photo.name}</span>
    <span>{photo.time}</span>
  </span>
</button>

<style>
  .photo-tile {
    position: relative;
    display: block;
    width: 100%;
    aspect-ratio: 4 / 3;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface-warm);
    padding: 0;
    transition: box-shadow 0.15s ease, opacity 0.15s ease;
  }

  .photo-tile:hover {
    box-shadow: var(--shadow-soft);
  }

  .photo-tile.selected {
    box-shadow: 0 0 0 2px var(--accent);
  }

  .photo-tile.culled {
    opacity: 0.44;
  }

  .photo-fill {
    position: absolute;
    inset: 0;
  }

  .photo-fill.image {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .photo-tile.culled .photo-fill {
    filter: grayscale(0.55);
  }

  .score-badge {
    position: absolute;
    top: 6px;
    left: 6px;
    border-radius: 3px;
    background: var(--surface);
    box-shadow: 0 0 0 1px var(--border);
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: 10px;
    font-variant-numeric: tabular-nums;
    font-weight: 600;
    padding: 2px 6px;
  }

  .score-badge.low {
    color: var(--danger);
  }

  .score-badge.mid {
    color: var(--warn);
  }

  .decision-mark {
    position: absolute;
    top: 6px;
    right: 6px;
    display: flex;
    align-items: center;
    justify-content: center;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    color: var(--accent-on);
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 600;
  }

  .decision-mark.keep {
    background: var(--success);
  }

  .decision-mark.cull {
    background: var(--danger);
  }

  .decision-mark.auto {
    background: var(--meta);
  }

  .photo-info {
    position: absolute;
    right: 0;
    bottom: 0;
    left: 0;
    display: flex;
    justify-content: space-between;
    background: linear-gradient(to top, rgba(20, 20, 19, 0.86), transparent);
    color: var(--surface);
    font-family: var(--font-mono);
    font-size: 10px;
    font-variant-numeric: tabular-nums;
    letter-spacing: 0.2px;
    padding: 15px 8px 6px;
  }

  .seq {
    opacity: 0.82;
  }
</style>
