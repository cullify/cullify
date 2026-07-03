<script lang="ts">
  import type { ThirdPartyProviderConfig } from '$lib/backend';
  import { Button } from '$lib/components/ui/button';
  import * as Card from '$lib/components/ui/card';
  import { Input } from '$lib/components/ui/input';

  interface Props {
    provider: ThirdPartyProviderConfig;
    updateProvider: (
      providerId: string,
      field: 'name' | 'baseUrl' | 'apiKey' | 'model' | 'kind',
      value: string
    ) => void;
    selectProvider: (providerId: string) => void;
  }

  let { provider, updateProvider, selectProvider }: Props = $props();
</script>

<div class="provider-fields remote-provider-fields">
  <label>
    <span>供应商 ID</span>
    <Input value={provider.id} readonly />
  </label>
  <label>
    <span>类型</span>
    <Input value="@ai-sdk/openai-compatible" readonly />
  </label>
  <label>
    <span>名称</span>
    <Input value={provider.name} oninput={(event) => updateProvider(provider.id, 'name', event.currentTarget.value)} />
  </label>
  <label>
    <span>Base URL</span>
    <Input
      value={provider.baseUrl}
      placeholder="http://localhost:11434/v1"
      oninput={(event) => updateProvider(provider.id, 'baseUrl', event.currentTarget.value)}
    />
  </label>
  <label>
    <span>API Key</span>
    <Input
      type="password"
      value={provider.apiKey}
      placeholder="本地服务可留空"
      oninput={(event) => updateProvider(provider.id, 'apiKey', event.currentTarget.value)}
    />
  </label>
  <label>
    <span>Model ID</span>
    <Input
      value={provider.model}
      placeholder="llava:latest / gpt-4o-mini"
      oninput={(event) => updateProvider(provider.id, 'model', event.currentTarget.value)}
    />
  </label>
</div>

<div id="models" class="remote-models">
  <div class="model-library-title">
    <h4>默认模型</h4>
    <span>{provider.enabled ? '启用' : '停用'}</span>
  </div>
  <Card.Root class="remote-model-row" size="sm">
    <span class="model-icon">{(provider.model || '?').slice(0, 1).toUpperCase()}</span>
    <div>
      <strong>{provider.model || '尚未填写模型 ID'}</strong>
      <small>{provider.baseUrl || '尚未配置 API Host'}</small>
    </div>
    <Button variant="outline" size="sm" onclick={() => selectProvider(provider.id)}>使用</Button>
  </Card.Root>
</div>

<style>
  .provider-fields {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px 16px;
  }

  .provider-fields label {
    display: grid;
    gap: 6px;
    min-width: 0;
  }

  .provider-fields span {
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 10px;
    text-transform: uppercase;
  }

  .remote-provider-fields {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: color-mix(in oklch, var(--card) 96%, var(--muted));
    padding: 16px;
  }

  :global(input[readonly]) {
    color: var(--meta);
    cursor: default;
  }

  .remote-models {
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

  :global(.remote-model-row) {
    display: grid;
    grid-template-columns: 34px minmax(0, 1fr) auto;
    align-items: center;
    gap: 12px;
    width: 100%;
    min-height: 68px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--card);
    padding: 12px 18px;
    text-align: left;
  }

  :global(.remote-model-row strong),
  :global(.remote-model-row small) {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  :global(.remote-model-row strong) {
    color: var(--fg);
    font-size: 14px;
    font-weight: 700;
  }

  :global(.remote-model-row small) {
    color: var(--muted-foreground);
    font-family: var(--font-mono);
    font-size: 10px;
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

  @media (max-width: 1000px) {
    .provider-fields {
      grid-template-columns: 1fr;
    }

    :global(.remote-model-row) {
      grid-template-columns: 34px minmax(0, 1fr);
    }

    :global(.remote-model-row [data-slot='button']) {
      grid-column: 2;
      justify-self: start;
    }
  }
</style>
