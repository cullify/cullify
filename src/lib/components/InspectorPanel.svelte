<script lang="ts">
  import type { Decision, Photo } from '$lib/types';

  type ShortcutBinding = { id: string; keys: string[] };

  interface Props {
    photo: Photo;
    index: number;
    total: number;
    shortcuts: ShortcutBinding[];
    onDecision: (decision: Decision) => void;
  }

  const fallbackShortcuts: Record<string, string[]> = {
    nav: ['Left', 'Right'],
    keep: ['K'],
    cull: ['X'],
    skip: ['S'],
    all: ['Up', 'Down'],
    undo: ['Cmd', 'Z'],
    fullscreen: ['F'],
    grid: ['G']
  };

  let { photo, index, total, shortcuts, onDecision }: Props = $props();

  const hintRows = $derived.by(() => [
    { label: '上一张', keys: keyAt('nav', 0) },
    { label: '下一张', keys: keyAt('nav', 1) },
    { label: '全屏灯箱', keys: keysFor('fullscreen') },
    { label: '网格切换', keys: keysFor('grid') },
    { label: '撤销', keys: keysFor('undo'), separator: '+' },
    { label: '全要 / 全不要', keys: keysFor('all'), separator: '/' }
  ]);

  function keysFor(id: string) {
    return shortcuts.find((shortcut) => shortcut.id === id)?.keys ?? fallbackShortcuts[id] ?? [];
  }

  function keyAt(id: string, index: number) {
    const key = keysFor(id)[index];
    return key ? [key] : [];
  }

  function shortcutLabel(id: string) {
    return keysFor(id).map(displayKey).join(' + ');
  }

  function displayKey(key: string) {
    if (key === 'Left') return '←';
    if (key === 'Right') return '→';
    if (key === 'Up') return '↑';
    if (key === 'Down') return '↓';
    if (key === 'Cmd') return '⌘';
    if (key === 'Space') return 'Space';
    return key;
  }
</script>

<section class="inspector-panel">
  <div class="photo-strip">
    <div class="preview" style:background={photo.palette}>
      {#if photo.sourceUrl}
        <img src={photo.sourceUrl} alt="" />
      {/if}
    </div>
    <div class="photo-heading">
      <p class="insp-eyebrow">当前照片 · {String(index + 1).padStart(2, '0')} / {total}</p>
      <h2 class="insp-title">{photo.name}</h2>
      <p class="insp-sub">{photo.title} · {photo.time} · {photo.fileSize}</p>
      <div class="exif-chips">
        <span>{photo.camera}</span>
        <span>{photo.lens}</span>
        <span>{photo.focal}</span>
        <span>{photo.aperture}</span>
        <span>{photo.shutter}</span>
        <span>ISO {photo.iso}</span>
        <span>{photo.size}</span>
        <span>{photo.group}</span>
      </div>
    </div>
  </div>

  <div class="score-panel">
    <div class="score-num">{photo.score}<small>/100</small></div>
    <div class="score-bars">
      <div class="score-bar">
        <span class="lbl">清晰度</span>
        <div class="track"><div class="fill" style:width={`${photo.clarity * 100}%`}></div></div>
        <span class="v">{photo.clarity.toFixed(2)}</span>
      </div>
      <div class="score-bar">
        <span class="lbl">曝光</span>
        <div class="track"><div class="fill" style:width={`${photo.exposure * 100}%`}></div></div>
        <span class="v">{photo.exposure.toFixed(2)}</span>
      </div>
      <div class="score-bar">
        <span class="lbl">构图</span>
        <div class="track"><div class="fill" style:width={`${photo.composition * 100}%`}></div></div>
        <span class="v">{photo.composition.toFixed(2)}</span>
      </div>
      <div class="score-bar">
        <span class="lbl">人脸</span>
        <div class="track"><div class="fill" style:width={photo.score > 70 ? '92%' : '54%'}></div></div>
        <span class="v">{photo.faceScore}</span>
      </div>
    </div>
  </div>

  <div class="reason-panel">
    <p class="insp-eyebrow">分析说明</p>
    <p>{photo.reason}</p>
  </div>

  <div class="action-panel">
    <div class="decisions">
      <button type="button" class="dec-btn keep" onclick={() => onDecision('keep')}>保留<span>{shortcutLabel('keep')}</span></button>
      <button type="button" class="dec-btn cull" onclick={() => onDecision('cull')}>淘汰<span>{shortcutLabel('cull')}</span></button>
      <button type="button" class="dec-btn skip" onclick={() => onDecision(null)}>跳过<span>{shortcutLabel('skip')}</span></button>
    </div>

    <div class="kbd-hints">
      {#each hintRows as row (row.label)}
        <div>
          <span>{row.label}</span>
          <span class="key-chord">
            {#each row.keys as key, keyIndex (`${row.label}-${key}-${keyIndex}`)}
              {#if keyIndex > 0}<span class="key-separator">{row.separator ?? '+'}</span>{/if}
              <span class="kbd">{displayKey(key)}</span>
            {/each}
          </span>
        </div>
      {/each}
    </div>
  </div>
</section>

<style>
  .inspector-panel {
    display: grid;
    grid-template-columns: minmax(280px, 1.45fr) minmax(220px, 0.8fr) minmax(260px, 1fr) minmax(250px, 0.85fr);
    gap: 14px;
    min-height: 170px;
    background: var(--surface);
    padding: 14px 18px;
  }

  .photo-strip {
    display: grid;
    grid-template-columns: 140px minmax(0, 1fr);
    gap: 14px;
    min-width: 0;
  }

  .photo-heading {
    min-width: 0;
  }

  .insp-eyebrow {
    margin: 0 0 10px;
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 1.1px;
    text-transform: uppercase;
  }

  .insp-title {
    overflow: hidden;
    margin: 0 0 6px;
    color: var(--fg);
    font-family: var(--font-display);
    font-size: 17px;
    font-weight: 500;
    line-height: 1.35;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .insp-sub {
    overflow: hidden;
    margin: 0;
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .preview {
    aspect-ratio: 4 / 3;
    overflow: hidden;
    border-radius: 6px;
    background: var(--surface-warm);
  }

  .preview img {
    display: block;
    width: 100%;
    height: 100%;
    object-fit: contain;
    background: var(--surface-strong);
  }

  .exif-chips {
    display: flex;
    flex-wrap: wrap;
    gap: 5px;
    margin-top: 12px;
  }

  .exif-chips span {
    max-width: 170px;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 4px;
    background: var(--bg);
    color: var(--fg-2);
    font-family: var(--font-mono);
    font-size: 10px;
    font-variant-numeric: tabular-nums;
    padding: 3px 6px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .score-panel {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .score-num {
    color: var(--accent);
    font-family: var(--font-display);
    font-size: 36px;
    font-variant-numeric: tabular-nums;
    line-height: 1;
  }

  .score-num small {
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 14px;
  }

  .score-bars {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: 5px;
  }

  .score-bar {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
  }

  .lbl,
  .v {
    font-family: var(--font-mono);
    font-size: 10px;
    font-variant-numeric: tabular-nums;
  }

  .lbl {
    width: 48px;
    color: var(--muted);
  }

  .track {
    flex: 1;
    height: 4px;
    overflow: hidden;
    border-radius: 2px;
    background: var(--border);
  }

  .fill {
    height: 100%;
    background: var(--accent);
  }

  .v {
    width: 34px;
    color: var(--fg-2);
    text-align: right;
  }

  .reason-panel {
    min-width: 0;
  }

  .reason-panel p:last-child {
    display: -webkit-box;
    overflow: hidden;
    border: 1px solid var(--border);
    border-left: 2px solid var(--accent);
    border-radius: 0 6px 6px 0;
    background: var(--bg);
    color: var(--muted);
    font-size: 13px;
    line-height: 1.6;
    margin: 0;
    padding: 12px 14px;
    -webkit-box-orient: vertical;
    -webkit-line-clamp: 4;
    line-clamp: 4;
  }

  .action-panel {
    display: flex;
    min-width: 0;
    flex-direction: column;
    gap: 10px;
  }

  .decisions {
    display: grid;
    grid-template-columns: 1fr 1fr 1fr;
    gap: 6px;
  }

  .dec-btn {
    border-radius: 6px;
    color: var(--accent-on);
    font-size: 12px;
    font-weight: 700;
    padding: 9px 8px;
  }

  .dec-btn span {
    display: block;
    margin-top: 3px;
    font-family: var(--font-mono);
    font-size: 9px;
    opacity: 0.7;
  }

  .dec-btn.keep {
    background: var(--success);
  }

  .dec-btn.cull {
    background: var(--danger);
  }

  .dec-btn.skip {
    background: var(--surface-warm);
    color: var(--fg-2);
    box-shadow: 0 0 0 1px var(--border);
  }

  .kbd-hints {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 6px 14px;
    min-width: 0;
    color: var(--meta);
    font-size: 11px;
  }

  .kbd-hints > div {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
  }

  .key-chord {
    display: inline-flex;
    align-items: center;
    gap: 4px;
  }

  .key-separator {
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 9px;
  }

  @media (max-width: 1320px) {
    .inspector-panel {
      grid-template-columns: minmax(280px, 1.3fr) minmax(260px, 1fr);
    }
  }

  @media (max-width: 900px) {
    .inspector-panel {
      grid-template-columns: 1fr;
    }

    .photo-strip {
      grid-template-columns: 112px minmax(0, 1fr);
    }
  }
</style>
