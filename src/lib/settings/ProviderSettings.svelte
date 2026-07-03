<script lang="ts">
  import type { HuggingFaceVisionModel, ModelDownloadProgress, ThirdPartyProviderConfig } from '$lib/backend';
  import { Alert, AlertDescription } from '$lib/components/ui/alert';
  import { Badge } from '$lib/components/ui/badge';
  import { Button } from '$lib/components/ui/button';
  import { Input } from '$lib/components/ui/input';
  import { Switch } from '$lib/components/ui/switch';
  import LocalModelLibrary from '$lib/settings/LocalModelLibrary.svelte';
  import RemoteProviderPanel from '$lib/settings/RemoteProviderPanel.svelte';
  import {
    providerInitials,
    providerStatusLabel,
    providerStatusText,
    selectedProviderSummary,
    type CatalogStatus
  } from '$lib/settings/modelHelpers';
  import PlusIcon from '@lucide/svelte/icons/plus';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import XIcon from '@lucide/svelte/icons/x';

  interface ProviderDraft {
    name: string;
    baseUrl: string;
    model: string;
    apiKey: string;
  }

  interface Props {
    activeProvider: string;
    activeModelId: string;
    activeProviderName: string;
    activeProviderConfig: ThirdPartyProviderConfig | null;
    activeLocalModelId: string;
    appDataDir: string;
    filteredThirdPartyProviders: ThirdPartyProviderConfig[];
    providerSearch: string;
    showProviderDraft: boolean;
    providerDraft: ProviderDraft;
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
    providerModelSummary: (providerId: string) => string;
    setProviderSearch: (value: string) => void;
    setProviderDraftField: (field: keyof ProviderDraft, value: string) => void;
    showDraft: () => void;
    cancelDraft: () => void;
    addProvider: () => void;
    removeProvider: (providerId: string) => void;
    toggleProvider: (providerId: string) => void;
    selectProvider: (providerId: string) => void;
    updateProvider: (
      providerId: string,
      field: 'name' | 'baseUrl' | 'apiKey' | 'model' | 'kind',
      value: string
    ) => void;
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
    activeProvider,
    activeModelId,
    activeProviderName,
    activeProviderConfig,
    activeLocalModelId,
    appDataDir,
    filteredThirdPartyProviders,
    providerSearch,
    showProviderDraft,
    providerDraft,
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
    providerModelSummary,
    setProviderSearch,
    setProviderDraftField,
    showDraft,
    cancelDraft,
    addProvider,
    removeProvider,
    toggleProvider,
    selectProvider,
    updateProvider,
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

<section id="provider" class="section">
  <div class="settings-head">
    <h2>模型供应商</h2>
    <span>{activeProvider} / {activeModelId}</span>
  </div>
  <p class="provider-intro">
    本地模型直接下载到设备；第三方供应商只维护 API Host、Key 和默认模型。
  </p>

  <div class="provider-workspace">
    <aside class="provider-directory" aria-label="模型供应商列表">
      <div class="directory-search">
        <Input
          value={providerSearch}
          placeholder="搜索供应商、模型或地址"
          aria-label="搜索模型供应商"
          oninput={(event) => setProviderSearch(event.currentTarget.value)}
        />
      </div>

      <Button
        variant="ghost"
        class={['provider-list-item', activeProvider === 'llama.cpp' && 'active'].filter(Boolean).join(' ')}
        aria-pressed={activeProvider === 'llama.cpp'}
        onclick={() => selectProvider('llama.cpp')}
      >
        <span class="provider-avatar local">ll</span>
        <span class="provider-list-copy">
          <strong>本地模型</strong>
          <small>{providerModelSummary('llama.cpp')}</small>
        </span>
        <Badge class="provider-pill local" variant="secondary">LOCAL</Badge>
      </Button>

      <div class="directory-divider">
        <span>第三方供应商</span>
        <span>{filteredThirdPartyProviders.length}</span>
      </div>

      {#each filteredThirdPartyProviders as provider (provider.id)}
        <Button
          variant="ghost"
          class={['provider-list-item', activeProvider === provider.id && 'active'].filter(Boolean).join(' ')}
          aria-pressed={activeProvider === provider.id}
          onclick={() => selectProvider(provider.id)}
        >
          <span class="provider-avatar">{providerInitials(provider)}</span>
          <span class="provider-list-copy">
            <strong>{provider.name || provider.id}</strong>
            <small>{selectedProviderSummary(provider)}</small>
          </span>
          <Badge
            class={['provider-pill', provider.enabled && provider.model && 'on'].filter(Boolean).join(' ')}
            variant={provider.enabled && provider.model ? 'secondary' : 'outline'}
          >
            {providerStatusLabel(provider)}
          </Badge>
        </Button>
      {/each}

      {#if showProviderDraft}
        <div class="provider-draft">
          <label>
            <span>名称</span>
            <Input
              value={providerDraft.name}
              placeholder="例如 SiliconFlow"
              oninput={(event) => setProviderDraftField('name', event.currentTarget.value)}
            />
          </label>
          <label>
            <span>API Host</span>
            <Input
              value={providerDraft.baseUrl}
              placeholder="https://api.example.com/v1"
              oninput={(event) => setProviderDraftField('baseUrl', event.currentTarget.value)}
            />
          </label>
          <label>
            <span>默认模型</span>
            <Input
              value={providerDraft.model}
              placeholder="provider/model-name"
              oninput={(event) => setProviderDraftField('model', event.currentTarget.value)}
            />
          </label>
          <label>
            <span>API Key</span>
            <Input
              type="password"
              value={providerDraft.apiKey}
              placeholder="可选"
              oninput={(event) => setProviderDraftField('apiKey', event.currentTarget.value)}
            />
          </label>
          <div class="draft-actions">
            <Button variant="outline" onclick={cancelDraft}>
              <XIcon data-icon="inline-start" aria-hidden="true" />
              取消
            </Button>
            <Button onclick={addProvider}>
              <PlusIcon data-icon="inline-start" aria-hidden="true" />
              添加
            </Button>
          </div>
        </div>
      {:else}
        <Button variant="outline" class="provider-add-button" onclick={showDraft}>
          <PlusIcon data-icon="inline-start" aria-hidden="true" />
          添加供应商
        </Button>
      {/if}
    </aside>

    <div class="provider-panel">
      <div class="provider-panel-head">
        <div>
          <h3>{activeProviderName}</h3>
          <p>
            {activeProvider === 'llama.cpp'
              ? '由 llama.cpp 驱动 · 下载来源由 Hugging Face 提供'
              : activeProviderConfig
                ? providerStatusText(activeProviderConfig)
                : '未选择供应商'}
          </p>
        </div>
        {#if activeProviderConfig}
          <div class="provider-head-actions">
            <Switch
              aria-label="切换供应商启用状态"
              checked={activeProviderConfig.enabled}
              onclick={() => toggleProvider(activeProviderConfig.id)}
            />
            <Button variant="destructive" size="sm" onclick={() => removeProvider(activeProviderConfig.id)}>
              <Trash2Icon data-icon="inline-start" aria-hidden="true" />
              删除
            </Button>
          </div>
        {/if}
      </div>

      {#if activeProvider === 'llama.cpp'}
        <LocalModelLibrary
          {appDataDir}
          {activeLocalModelId}
          {downloadingModelId}
          {modelCatalog}
          {visibleCatalogModels}
          {modelCatalogStatus}
          {modelCatalogTotal}
          {modelCatalogHasMore}
          {modelCatalogCacheDate}
          {modelCatalogSearch}
          {isCatalogRefreshing}
          {isLoadingMoreModels}
          {downloadMessage}
          {progressForModel}
          {progressButtonLabel}
          {modelStatusText}
          {resumableBytes}
          {modelActionDisabled}
          {setModelCatalogSearch}
          {refreshCatalog}
          {loadMoreModels}
          {downloadCatalogModel}
          {cancelActiveModelDownload}
        />
      {:else if activeProviderConfig}
        <RemoteProviderPanel provider={activeProviderConfig} {updateProvider} {selectProvider} />
      {:else}
        <Alert class="empty-provider">
          <strong>未选择可用供应商</strong>
          <AlertDescription>请选择 llama.cpp 或添加一个 OpenAI-compatible provider。</AlertDescription>
        </Alert>
      {/if}
    </div>
  </div>
</section>

<style>
  .section {
    margin-bottom: 48px;
  }

  .settings-head {
    display: flex;
    align-items: baseline;
    justify-content: space-between;
    gap: 18px;
    border-bottom: 1px solid var(--border);
    margin-bottom: 18px;
    padding-bottom: 10px;
  }

  .settings-head h2 {
    margin: 0;
    color: var(--fg);
    font-family: var(--font-display);
    font-size: 18px;
    font-weight: 500;
  }

  .settings-head span {
    color: var(--muted-foreground);
    font-size: 12px;
  }

  .provider-intro {
    max-width: 68ch;
    margin: -8px 0 18px;
    color: var(--muted-foreground);
    font-size: 12px;
    line-height: 1.65;
  }

  .provider-workspace {
    display: grid;
    grid-template-columns: minmax(270px, 320px) minmax(0, 1fr);
    min-height: 560px;
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--card);
    box-shadow: var(--shadow-soft);
  }

  .provider-directory {
    display: grid;
    grid-auto-rows: max-content;
    gap: 8px;
    border-right: 1px solid var(--border);
    background: color-mix(in oklch, var(--card) 92%, var(--muted));
    padding: 14px 12px;
  }

  .directory-search :global(input) {
    width: 100%;
    border-radius: 999px;
    background: var(--card);
    padding: 10px 14px;
  }

  .directory-divider,
  .draft-actions,
  .provider-head-actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
  }

  .directory-divider {
    margin: 8px 4px 2px;
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 10px;
    text-transform: uppercase;
  }

  :global(.provider-list-item) {
    display: grid;
    grid-template-columns: 34px minmax(0, 1fr) auto;
    align-items: center;
    gap: 11px;
    min-height: 54px;
    border: 1px solid transparent;
    border-radius: 8px;
    background: transparent;
    padding: 8px 10px;
    text-align: left;
  }

  :global(.provider-list-item:hover),
  :global(.provider-list-item.active) {
    border-color: color-mix(in oklch, var(--success) 32%, var(--border));
    background: var(--card);
  }

  :global(.provider-list-item.active) {
    box-shadow: var(--shadow-soft);
  }

  .provider-avatar {
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

  .provider-avatar.local {
    background: color-mix(in oklch, var(--success) 13%, var(--card));
    color: var(--success);
  }

  .provider-list-copy {
    min-width: 0;
  }

  .provider-list-copy strong,
  .provider-list-copy small {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .provider-list-copy strong {
    color: var(--fg);
    font-size: 14px;
    font-weight: 700;
  }

  .provider-list-copy small {
    color: var(--muted-foreground);
    font-family: var(--font-mono);
    font-size: 10px;
  }

  :global(.provider-pill) {
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--secondary);
    color: var(--muted-foreground);
    font-family: var(--font-mono);
    font-size: 10px;
    padding: 3px 8px;
  }

  :global(.provider-pill.on),
  :global(.provider-pill.local) {
    border-color: color-mix(in oklch, var(--success) 45%, var(--border));
    background: color-mix(in oklch, var(--success) 13%, var(--card));
    color: var(--success);
  }

  :global(.provider-add-button) {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: 40px;
    border: 1px solid var(--border);
    border-radius: 999px;
    background: var(--card);
    color: var(--fg-2);
    font-weight: 700;
  }

  .provider-workspace :global([data-slot='button']:focus) {
    outline: none;
  }

  .provider-workspace :global([data-slot='button']:focus-visible),
  .provider-workspace :global(input:focus-visible) {
    outline: 2px solid color-mix(in oklch, var(--success) 28%, transparent);
    outline-offset: 2px;
  }

  .provider-draft {
    display: grid;
    gap: 10px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--card);
    padding: 12px;
  }

  .provider-draft label {
    display: grid;
    gap: 6px;
    min-width: 0;
  }

  .provider-draft span {
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 10px;
    text-transform: uppercase;
  }

  .provider-panel {
    display: grid;
    align-content: start;
    gap: 18px;
    min-width: 0;
    background: var(--card);
    padding: 24px 28px 30px;
  }

  .provider-panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    min-height: 52px;
    border-bottom: 1px solid var(--border);
    padding-bottom: 18px;
  }

  .provider-panel-head h3 {
    margin: 0 0 4px;
    color: var(--fg);
    font-size: 18px;
    font-weight: 800;
  }

  .provider-panel-head p {
    margin: 0;
    color: var(--muted-foreground);
    font-size: 12px;
    overflow-wrap: anywhere;
  }

  :global(.empty-provider) {
    display: grid;
    gap: 14px;
    border: 1px solid color-mix(in oklch, var(--success) 24%, var(--border));
    border-radius: 8px;
    background: color-mix(in oklch, var(--success) 6%, var(--card));
    padding: 16px;
  }

  @media (max-width: 1000px) {
    .provider-workspace {
      grid-template-columns: 1fr;
    }

    .provider-directory {
      border-right: 0;
      border-bottom: 1px solid var(--border);
    }

    .provider-panel-head,
    .settings-head {
      align-items: flex-start;
      flex-direction: column;
    }
  }
</style>
