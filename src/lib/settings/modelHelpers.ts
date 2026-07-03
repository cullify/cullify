import type { HuggingFaceVisionModel, LocalModelConfig, ThirdPartyProviderConfig } from '$lib/backend';

export type CatalogStatus = 'ready' | 'fallback';
export type NumberSettingField = 'blur' | 'exposure' | 'cull' | 'threads';

export function cloneLocalModels(models: LocalModelConfig[]) {
  return models.map((model) => ({ ...model, localPath: model.localPath ?? null }));
}

export function cloneProviders(nextProviders: ThirdPartyProviderConfig[]) {
  return nextProviders.map((provider) => ({ ...provider }));
}

export function normalizeProvider(providerId: string, currentProviders: ThirdPartyProviderConfig[]) {
  if (providerId === 'llama.cpp' || providerId === 'builtin') return 'llama.cpp';
  if (providerId === 'ollama') return 'ollama-local';
  if (providerId === 'openai') return 'openai-compatible';
  if (currentProviders.some((provider) => provider.id === providerId)) return providerId;
  return 'llama.cpp';
}

export function uniqueId(seed: string, existing: string[], fallback: string) {
  const base =
    seed
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, '-')
      .replace(/^-+|-+$/g, '') || fallback;
  let candidate = base;
  let index = 2;
  while (existing.includes(candidate)) {
    candidate = `${base}-${index}`;
    index += 1;
  }
  return candidate;
}

export function legacyProviderId(providerId: string) {
  return providerId === 'llama.cpp' ? 'builtin' : providerId;
}

export function formatBytes(value: number) {
  if (value < 1024 * 1024) return `${Math.max(1, Math.round(value / 1024))} KB`;
  if (value < 1024 * 1024 * 1024) return `${(value / 1024 / 1024).toFixed(1)} MB`;
  return `${(value / 1024 / 1024 / 1024).toFixed(2)} GB`;
}

export function selectedProviderSummary(provider: ThirdPartyProviderConfig) {
  if (!provider.enabled) return '已停用';
  if (!provider.model) return '未填写模型名';
  return provider.model;
}

export function providerInitials(provider: ThirdPartyProviderConfig) {
  return (provider.name || provider.id).slice(0, 2).toUpperCase();
}

export function providerStatusLabel(provider: ThirdPartyProviderConfig) {
  if (!provider.enabled) return 'OFF';
  if (!provider.model) return 'SETUP';
  return 'ON';
}

export function providerStatusText(provider: ThirdPartyProviderConfig) {
  if (!provider.enabled) return '当前停用';
  if (!provider.baseUrl) return '需要填写 API Host';
  if (!provider.model) return '需要选择模型';
  return '可用于专家模式';
}

export function catalogModelFromLocalModel(model: LocalModelConfig): HuggingFaceVisionModel {
  const repoId = model.downloadUrl.match(/huggingface\.co\/([^/]+\/[^/]+)/)?.[1] ?? 'local/models';
  return {
    id: model.id,
    repoId,
    name: model.name,
    author: repoId.split('/')[0] ?? 'local',
    task: model.downloadUrl.includes('huggingface.co') ? 'image-text-to-text' : 'local',
    license: '未标注',
    fileName: model.fileName,
    downloadUrl: model.downloadUrl,
    downloads: 0,
    likes: 0,
    lastModified: 'unknown',
    downloaded: model.downloaded,
    updateAvailable: false,
    localPath: model.localPath,
    partialDownloadedBytes: 0,
    partialPath: null
  };
}

export function modelSourceText(model: HuggingFaceVisionModel) {
  return `${model.repoId} · ${model.fileName}`;
}

export function modelMetaText(model: HuggingFaceVisionModel) {
  return [model.task, model.license, `${model.likes} likes`, formatModelDate(model.lastModified)]
    .filter(Boolean)
    .join(' · ');
}

export function validDownloadUrl(value: string) {
  return value.trim().startsWith('https://') || value.trim().startsWith('http://');
}

export function formatCompactNumber(value: number) {
  if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
  if (value >= 1_000) return `${(value / 1_000).toFixed(1)}k`;
  return `${value}`;
}

export function formatModelDate(value: string) {
  if (!value || value === 'unknown') return '未知';
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return '未知';
  return date.toISOString().slice(0, 10);
}

export function upsertLocalModelFromCatalog(
  localModels: LocalModelConfig[],
  model: HuggingFaceVisionModel,
  localPath: string
) {
  const nextModel: LocalModelConfig = {
    id: model.id,
    name: model.name,
    fileName: model.fileName,
    size: 'GGUF',
    speed: 'llama.cpp',
    downloadUrl: model.downloadUrl,
    localPath,
    downloaded: true
  };
  if (localModels.some((item) => item.id === model.id || item.fileName === model.fileName)) {
    return localModels.map((item) => (item.id === model.id || item.fileName === model.fileName ? nextModel : item));
  }
  return [...localModels, nextModel];
}
