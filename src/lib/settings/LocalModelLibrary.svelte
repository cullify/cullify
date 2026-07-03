<script lang="ts">
  import type { HuggingFaceVisionModel, ModelDownloadProgress } from '$lib/backend';
  import { Button } from '$lib/components/ui/button';
  import * as Card from '$lib/components/ui/card';
  import { Input } from '$lib/components/ui/input';
  import { Progress } from '$lib/components/ui/progress';
  import DownloadIcon from '@lucide/svelte/icons/download';
  import RefreshCwIcon from '@lucide/svelte/icons/refresh-cw';
  import type { CatalogStatus } from '$lib/settings/modelHelpers';
  import { modelMetaText, modelSourceText } from '$lib/settings/modelHelpers';

  interface Props {
    appDataDir: string;
    activeLocalModelId: string;
    downloadingModelId: string;
    modelCatalog: HuggingFaceVisionModel[];
    visibleCatalogModels: HuggingFaceVisionModel[];
    modelCatalogStatus: CatalogStatus;
    modelCatalogTotal: number;
    modelCatalogHasMore: boolean;
    modelCatalogCacheDate: string;
    modelCatalogSearch: string;
    isCatalogRefreshing: boolean;
    isLoadingMoreModels: boolean;
    downloadMessage: string;
    progressForModel: (modelId: string) => ModelDownloadProgress | null;
    progressButtonLabel: (model: HuggingFaceVisionModel, progress: ModelDownloadProgress | null) => string;
    modelStatusText: (model: HuggingFaceVisionModel, partialBytes: number) => string;
    resumableBytes: (model: HuggingFaceVisionModel) => number;
    modelActionDisabled: (model: HuggingFaceVisionModel) => boolean;
    setModelCatalogSearch: (value: string) => void;
    refreshCatalog: () => void;
    loadMoreModels: () => void;
    downloadCatalogModel: (model: HuggingFaceVisionModel) => void;
    cancelActiveModelDownload: (modelId: string) => void;
  }

  let {
    appDataDir,
    activeLocalModelId,
    downloadingModelId,
    modelCatalog,
    visibleCatalogModels,
    modelCatalogStatus,
    modelCatalogTotal,
    modelCatalogHasMore,
    modelCatalogCacheDate,
    modelCatalogSearch,
    isCatalogRefreshing,
    isLoadingMoreModels,
    downloadMessage,
    progressForModel,
    progressButtonLabel,
    modelStatusText,
    resumableBytes,
    modelActionDisabled,
    setModelCatalogSearch,
    refreshCatalog,
    loadMoreModels,
    downloadCatalogModel,
    cancelActiveModelDownload
  }: Props = $props();
</script>

<Card.Root class="local-model-summary" size="sm">
  <div>
    <span>保存位置</span>
    <strong>{`${appDataDir}/models`}</strong>
  </div>
  <div>
    <span>下载来源</span>
    <strong>Hugging Face</strong>
  </div>
  <div>
    <span>任务状态</span>
    <strong>{downloadingModelId ? '下载中' : '空闲'}</strong>
  </div>
</Card.Root>

<div id="models" class="model-library">
  <div class="model-library-title">
    <h4>模型库</h4>
    <span>
      {#if isCatalogRefreshing && !modelCatalog.length}
        读取中
      {:else if modelCatalogStatus === 'fallback'}
        本地列表
      {:else}
        {visibleCatalogModels.length} / {modelCatalogTotal} · {modelCatalogCacheDate}
      {/if}
    </span>
  </div>

  <div class="model-catalog-toolbar">
    <Input
      value={modelCatalogSearch}
      placeholder="搜索 Hugging Face 视觉模型"
      aria-label="搜索 Hugging Face 视觉模型"
      oninput={(event) => setModelCatalogSearch(event.currentTarget.value)}
    />
    <Button variant="outline" size="sm" disabled={isCatalogRefreshing} onclick={refreshCatalog}>
      <RefreshCwIcon data-icon="inline-start" aria-hidden="true" />
      {isCatalogRefreshing ? '刷新中' : '刷新缓存'}
    </Button>
  </div>

  <div class="hf-model-table" role="list" aria-label="Hugging Face 视觉模型">
    {#each visibleCatalogModels as model (model.id)}
      {@const progress = progressForModel(model.id)}
      {@const partialBytes = resumableBytes(model)}
      <Card.Root
        class={['hf-model-row', model.id === activeLocalModelId && 'active'].filter(Boolean).join(' ')}
        role="listitem"
        size="sm"
      >
        <div class="hf-model-name">
          <span class="model-icon">{model.name.slice(0, 1)}</span>
          <span class="hf-model-copy">
            <strong>{model.name}</strong>
            <small>{modelSourceText(model)}</small>
            <em>{modelMetaText(model)}</em>
          </span>
        </div>
        <span class="hf-model-action">
          <small>{modelStatusText(model, partialBytes)}</small>
          <Button
            size="sm"
            variant={downloadingModelId === model.id ? 'destructive' : model.downloaded && !model.updateAvailable ? 'secondary' : 'outline'}
            class={[
              'model-action',
              model.downloaded && !model.updateAvailable && 'done',
              model.updateAvailable && 'update',
              downloadingModelId === model.id && 'cancel'
            ].filter(Boolean).join(' ')}
            disabled={modelActionDisabled(model)}
            onclick={() => (downloadingModelId === model.id ? cancelActiveModelDownload(model.id) : downloadCatalogModel(model))}
          >
            {#if downloadingModelId !== model.id && (!model.downloaded || model.updateAvailable)}
              <DownloadIcon data-icon="inline-start" aria-hidden="true" />
            {/if}
            {progressButtonLabel(model, progress)}
          </Button>
          {#if progress}
            <Progress class="download-progress-track" aria-label="下载进度" value={progress.percent ?? 0} />
          {/if}
        </span>
      </Card.Root>
    {:else}
      <div class="hf-model-empty">没有匹配的视觉模型。</div>
    {/each}
  </div>

  {#if modelCatalogHasMore && !modelCatalogSearch.trim()}
    <Button variant="outline" class="load-more-models" disabled={isLoadingMoreModels} onclick={loadMoreModels}>
      {isLoadingMoreModels ? '加载中' : `加载更多 · ${modelCatalog.length} / ${modelCatalogTotal}`}
    </Button>
  {/if}

  {#if downloadMessage}
    <p class="download-message">{downloadMessage}</p>
  {/if}
</div>

<style>
  :global(.local-model-summary) {
    display: grid;
    grid-template-columns: 1.45fr 0.8fr 0.65fr;
    gap: 1px;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--border);
  }

  :global(.local-model-summary div) {
    display: grid;
    gap: 5px;
    min-width: 0;
    background: color-mix(in oklch, var(--card) 92%, var(--muted));
    padding: 12px 14px;
  }

  :global(.local-model-summary span) {
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 10px;
    text-transform: uppercase;
  }

  :global(.local-model-summary strong) {
    overflow: hidden;
    color: var(--fg-2);
    font-size: 12px;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .model-library {
    display: grid;
    gap: 12px;
  }

  .model-library-title {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .model-library-title h4 {
    margin: 0;
    color: var(--fg);
    font-size: 16px;
    font-weight: 800;
  }

  .model-library-title span {
    color: var(--muted-foreground);
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .model-catalog-toolbar {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px;
  }

  .model-catalog-toolbar :global(input) {
    width: 100%;
    border-radius: 999px;
    background: var(--card);
    padding: 10px 14px;
  }

  .hf-model-table {
    display: grid;
    gap: 8px;
  }

  :global(.hf-model-row) {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 140px;
    align-items: center;
    gap: 18px;
    min-height: 82px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--card);
    padding: 12px 14px;
  }

  :global(.hf-model-row:hover),
  :global(.hf-model-row.active) {
    border-color: color-mix(in oklch, var(--success) 34%, var(--border));
    background: color-mix(in oklch, var(--success) 6%, var(--card));
  }

  :global(.hf-model-row.active) {
    box-shadow: inset 3px 0 0 var(--success), var(--shadow-soft);
  }

  .hf-model-name {
    display: grid;
    grid-template-columns: 34px minmax(0, 1fr);
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .model-icon {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    flex: 0 0 auto;
    border-radius: 50%;
    background: color-mix(in oklch, var(--primary) 12%, var(--card));
    color: var(--primary);
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
  }

  .hf-model-name strong,
  .hf-model-name small,
  .hf-model-copy em {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hf-model-name strong {
    color: var(--fg);
    font-size: 14px;
    font-weight: 800;
  }

  .hf-model-copy {
    min-width: 0;
  }

  .hf-model-name small,
  .hf-model-copy em {
    color: var(--muted-foreground);
    font-family: var(--font-mono);
    font-size: 10px;
  }

  .hf-model-copy em {
    margin-top: 3px;
    color: var(--meta);
    font-style: normal;
  }

  .hf-model-action {
    display: grid;
    justify-items: end;
    gap: 7px;
    min-width: 0;
  }

  .hf-model-action small {
    max-width: 140px;
    overflow: hidden;
    color: var(--fg-2);
    font-family: var(--font-mono);
    font-size: 10px;
    line-height: 1.2;
    text-align: right;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.download-progress-track) {
    width: 100%;
    height: 4px;
    overflow: hidden;
    border-radius: 999px;
    background: var(--muted);
  }

  :global(.download-progress-track [data-slot='progress-indicator']) {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: var(--success);
    transition: width 160ms ease;
  }

  .hf-model-empty {
    color: var(--meta);
    padding: 18px;
  }

  :global(.load-more-models) {
    width: 100%;
    min-height: 38px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--card);
    color: var(--fg-2);
    font-family: var(--font-mono);
    font-size: 11px;
  }

  :global(.load-more-models:disabled) {
    color: var(--meta);
    cursor: progress;
  }

  :global(.model-action) {
    border: 1px solid var(--border);
    border-radius: 4px;
    background: transparent;
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    padding: 5px 10px;
    text-transform: uppercase;
    white-space: pre-line;
  }

  :global(.model-action:disabled) {
    border-color: var(--border-soft);
    color: var(--meta);
    cursor: not-allowed;
  }

  :global(.model-action.done) {
    border-color: color-mix(in oklch, var(--success) 45%, var(--border));
    background: color-mix(in oklch, var(--success) 13%, var(--card));
    color: var(--success);
  }

  :global(.model-action.update) {
    border-color: color-mix(in oklch, var(--warn) 42%, var(--border));
    background: color-mix(in oklch, var(--warn) 12%, var(--card));
    color: var(--warn);
  }

  :global(.model-action.cancel) {
    border-color: color-mix(in oklch, var(--danger) 38%, var(--border));
    background: color-mix(in oklch, var(--danger) 10%, var(--card));
    color: var(--danger);
  }

  .download-message {
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 10px;
    margin: 10px 0 0;
    overflow-wrap: anywhere;
  }

  @media (max-width: 1000px) {
    :global(.local-model-summary) {
      grid-template-columns: 1fr;
    }

    :global(.hf-model-row) {
      grid-template-columns: 1fr;
      align-items: start;
      gap: 8px;
      padding: 14px;
    }

    .hf-model-action {
      justify-items: start;
      width: 100%;
    }

    .hf-model-action small {
      text-align: left;
    }

    .model-catalog-toolbar {
      grid-template-columns: 1fr;
    }
  }
</style>
