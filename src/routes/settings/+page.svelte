<script lang="ts">
  import { onDestroy, onMount } from 'svelte';
  import {
    defaultAppConfig,
    listenModelDownloadProgress,
    tryCancelModelDownload,
    tryDownloadModel,
    tryListHuggingFaceVisionModels,
    tryLoadAppConfig,
    trySaveAppConfig,
    type AppConfig,
    type DownloadModelSummary,
    type HuggingFaceVisionModel,
    type LocalModelConfig,
    type ModelDownloadProgress,
    type ThirdPartyProviderConfig
  } from '$lib/backend';
  import { shortcuts as seedShortcuts } from '$lib/mockData';
  import { getShellContext } from '$lib/shell.svelte';
  import type { Shortcut } from '$lib/types';

  let activeProvider = $state(defaultAppConfig.activeModelProviderId);
  let activeLocalModelId = $state(defaultAppConfig.activeLocalModelId);
  let localModels = $state<LocalModelConfig[]>(cloneLocalModels(defaultAppConfig.localModels));
  let thirdPartyProviders = $state<ThirdPartyProviderConfig[]>(cloneProviders(defaultAppConfig.thirdPartyProviders));
  let blurThreshold = $state(defaultAppConfig.blurThreshold);
  let exposureTolerance = $state(defaultAppConfig.exposureTolerance);
  let cullLine = $state(defaultAppConfig.cullLine);
  let vlmThreads = $state(defaultAppConfig.vlmThreads);
  let arenaTarget = $state(defaultAppConfig.arenaTarget);
  let autoGroup = $state(defaultAppConfig.autoGroup);
  let gpuMetal = $state(defaultAppConfig.gpuMetal);
  let shortcuts = $state<Shortcut[]>(seedShortcuts.map((shortcut) => ({ ...shortcut, keys: [...shortcut.keys] })));
  let editingShortcut = $state<string | null>(null);
  let dirtyFields = $state<string[]>([]);
  let configPath = $state('桌面环境可用');
  let appDataDir = $state('桌面环境可用');
  let isLoadingConfig = $state(true);
  let isSavingConfig = $state(false);
  let configError = $state('');
  let downloadingModelId = $state('');
  let cancelingModelId = $state('');
  let downloadMessage = $state('');
  let downloadProgress = $state<Record<string, ModelDownloadProgress>>({});
  let resumableDownloads = $state<Record<string, number>>({});
  let modelCatalog = $state<HuggingFaceVisionModel[]>([]);
  let modelCatalogStatus = $state<'ready' | 'fallback'>('fallback');
  let modelCatalogTotal = $state(0);
  let modelCatalogHasMore = $state(false);
  let modelCatalogCacheDate = $state('');
  let isCatalogRefreshing = $state(false);
  let isLoadingMoreModels = $state(false);
  let modelCatalogSearch = $state('');
  let providerSearch = $state('');
  let showProviderDraft = $state(false);
  let newProviderName = $state('');
  let newProviderBaseUrl = $state('http://localhost:11434/v1');
  let newProviderModel = $state('');
  let newProviderApiKey = $state('');

  const activeProviderConfig = $derived(thirdPartyProviders.find((provider) => provider.id === activeProvider) ?? null);
  const activeProviderName = $derived(providerName(activeProvider));
  const activeModelId = $derived(activeProvider === 'llama.cpp' ? activeLocalModelId : (activeProviderConfig?.model || '未指定'));
  const activeLocalModel = $derived(localModels.find((model) => model.id === activeLocalModelId) ?? null);
  const dirtyCount = $derived(dirtyFields.length);
  const shell = getShellContext();
  const filteredThirdPartyProviders = $derived.by(() => {
    const keyword = providerSearch.trim().toLowerCase();
    if (!keyword) return thirdPartyProviders;
    return thirdPartyProviders.filter((provider) =>
      [provider.name, provider.id, provider.baseUrl, provider.model].some((value) => value.toLowerCase().includes(keyword))
    );
  });
  const visibleCatalogModels = $derived.by(() => {
    const rows = modelCatalog.length ? modelCatalog : localModels.map(catalogModelFromLocalModel);
    const keyword = modelCatalogSearch.trim().toLowerCase();
    if (!keyword) return rows;
    return rows.filter((model) =>
      [model.name, model.repoId, model.fileName, model.task, model.author].some((value) =>
        value.toLowerCase().includes(keyword)
      )
    );
  });

  onMount(() => {
    void loadSavedConfig();
    void loadHuggingFaceCatalog();
  });

  onDestroy(() => {
    shell.resetPage();
  });

  $effect(() => {
    shell.configure({
      active: 'settings',
      title: '设置',
      subtitle: `${activeProviderName} · ${activeModelId}`,
      cullCount: '·',
      sidebarExtra: settingsSidebar,
      showDefaultSidebarDetails: false,
      footer: {
        photo: '照片分析待机',
        analysis: dirtyCount ? `${dirtyCount} 项配置未保存` : '配置已保存',
        model: downloadingModelId
          ? `模型下载 · ${downloadingModelId}`
          : isCatalogRefreshing
            ? '模型目录刷新中'
            : `当前模型 · ${activeModelId}`,
        resources: {
          cpu: isCatalogRefreshing || downloadingModelId ? 'CPU 网络任务' : 'CPU 待机',
          memory: modelCatalog.length ? `内存 模型缓存 ${modelCatalog.length}/${modelCatalogTotal}` : '内存 本地列表',
          gpu: gpuMetal ? '显存 Metal 已启用' : '显存 Metal 关闭'
        }
      }
    });
  });

  async function loadSavedConfig() {
    isLoadingConfig = true;
    configError = '';
    const envelope = await tryLoadAppConfig();
    if (envelope) {
      configPath = envelope.configPath;
      appDataDir = envelope.appDataDir;
      applyConfig(envelope.config);
    } else {
      configPath = '桌面环境可用';
      appDataDir = '桌面环境可用';
      applyConfig(defaultAppConfig);
    }
    isLoadingConfig = false;
  }

  function applyConfig(config: AppConfig) {
    localModels = cloneLocalModels(config.localModels?.length ? config.localModels : defaultAppConfig.localModels);
    thirdPartyProviders = cloneProviders(
      config.thirdPartyProviders?.length ? config.thirdPartyProviders : defaultAppConfig.thirdPartyProviders
    );
    activeProvider = normalizeProvider(config.activeModelProviderId || config.providerId, thirdPartyProviders);
    activeLocalModelId = config.activeLocalModelId || config.activeModelId || localModels[0]?.id || defaultAppConfig.activeLocalModelId;
    if (!localModels.some((model) => model.id === activeLocalModelId)) {
      activeLocalModelId = localModels[0]?.id ?? defaultAppConfig.activeLocalModelId;
    }
    blurThreshold = config.blurThreshold;
    exposureTolerance = config.exposureTolerance;
    cullLine = config.cullLine;
    vlmThreads = config.vlmThreads;
    arenaTarget = config.arenaTarget;
    autoGroup = config.autoGroup;
    gpuMetal = config.gpuMetal;

    shortcuts = seedShortcuts.map((shortcut) => ({
      ...shortcut,
      keys: config.shortcuts.find((item) => item.id === shortcut.id)?.keys ?? [...shortcut.keys]
    }));
    downloadMessage = '';
    dirtyFields = [];
  }

  function normalizeProvider(providerId: string, currentProviders = thirdPartyProviders) {
    if (providerId === 'llama.cpp' || providerId === 'builtin') return 'llama.cpp';
    if (providerId === 'ollama') return 'ollama-local';
    if (providerId === 'openai') return 'openai-compatible';
    if (currentProviders.some((provider) => provider.id === providerId)) return providerId;
    return 'llama.cpp';
  }

  function providerName(providerId: string) {
    if (providerId === 'llama.cpp') return '本地模型';
    return thirdPartyProviders.find((provider) => provider.id === providerId)?.name ?? '未配置';
  }

  function cloneLocalModels(models: LocalModelConfig[]) {
    return models.map((model) => ({ ...model, localPath: model.localPath ?? null }));
  }

  function cloneProviders(nextProviders: ThirdPartyProviderConfig[]) {
    return nextProviders.map((provider) => ({ ...provider }));
  }

  function uniqueId(seed: string, existing: string[], fallback: string) {
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

  function legacyProviderId(providerId: string) {
    return providerId === 'llama.cpp' ? 'builtin' : providerId;
  }

  function resolvedActiveModelId() {
    if (activeProvider === 'llama.cpp') return activeLocalModelId;
    return thirdPartyProviders.find((provider) => provider.id === activeProvider)?.model || '';
  }

  function selectedLocalModelPath(model: LocalModelConfig) {
    return model.localPath || `${appDataDir}/models/${model.fileName}`;
  }

  function formatBytes(value: number) {
    if (value < 1024 * 1024) return `${Math.max(1, Math.round(value / 1024))} KB`;
    if (value < 1024 * 1024 * 1024) return `${(value / 1024 / 1024).toFixed(1)} MB`;
    return `${(value / 1024 / 1024 / 1024).toFixed(2)} GB`;
  }

  function progressForModel(modelId: string) {
    return downloadProgress[modelId] ?? null;
  }

  function progressButtonLabel(model: HuggingFaceVisionModel, progress: ModelDownloadProgress | null) {
    if (downloadingModelId !== model.id) return modelActionLabel(model);
    if (cancelingModelId === model.id) return '取消中';
    return progress ? '取消下载' : '取消';
  }

  function progressDetail(progress: ModelDownloadProgress) {
    if (progress.totalBytes) return `${formatBytes(progress.downloadedBytes)} / ${formatBytes(progress.totalBytes)}`;
    return `${formatBytes(progress.downloadedBytes)} 已下载`;
  }

  function modelStatusText(model: HuggingFaceVisionModel, partialBytes: number) {
    if (downloadingModelId === model.id) {
      const progress = progressForModel(model.id);
      return progress ? progressDetail(progress) : '准备下载';
    }
    if (model.downloaded && !model.updateAvailable) return '本地可用';
    if (model.updateAvailable) return '可更新';
    if (partialBytes > 0) return `可继续 · ${formatBytes(partialBytes)}`;
    return `${formatCompactNumber(model.downloads)} downloads`;
  }

  function modelSourceText(model: HuggingFaceVisionModel) {
    return `${model.repoId} · ${model.fileName}`;
  }

  function modelMetaText(model: HuggingFaceVisionModel) {
    return [model.task, model.license, `${model.likes} likes`, formatModelDate(model.lastModified)]
      .filter(Boolean)
      .join(' · ');
  }

  function removeDownloadProgress(modelId: string) {
    const nextProgress = { ...downloadProgress };
    delete nextProgress[modelId];
    downloadProgress = nextProgress;
  }

  function rememberResumableDownload(modelId: string) {
    const bytes = progressForModel(modelId)?.downloadedBytes ?? 0;
    if (bytes <= 0) return;
    resumableDownloads = { ...resumableDownloads, [modelId]: bytes };
  }

  function forgetResumableDownload(modelId: string) {
    const nextDownloads = { ...resumableDownloads };
    delete nextDownloads[modelId];
    resumableDownloads = nextDownloads;
  }

  function selectedProviderSummary(provider: ThirdPartyProviderConfig) {
    if (!provider.enabled) return '已停用';
    if (!provider.model) return '未填写模型名';
    return provider.model;
  }

  function providerInitials(provider: ThirdPartyProviderConfig) {
    return (provider.name || provider.id).slice(0, 2).toUpperCase();
  }

  function providerStatusLabel(provider: ThirdPartyProviderConfig) {
    if (!provider.enabled) return 'OFF';
    if (!provider.model) return 'SETUP';
    return 'ON';
  }

  function providerStatusText(provider: ThirdPartyProviderConfig) {
    if (!provider.enabled) return '当前停用';
    if (!provider.baseUrl) return '需要填写 API Host';
    if (!provider.model) return '需要选择模型';
    return '可用于专家模式';
  }

  function providerModelSummary(providerId: string) {
    if (providerId === 'llama.cpp') return `llama.cpp 驱动 · ${activeLocalModel?.name ?? activeLocalModelId}`;
    return thirdPartyProviders.find((provider) => provider.id === providerId)?.model || '未指定模型';
  }

  function catalogModelFromLocalModel(model: LocalModelConfig): HuggingFaceVisionModel {
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

  function resumableBytes(model: HuggingFaceVisionModel) {
    if (model.downloaded && !model.updateAvailable) return 0;
    return resumableDownloads[model.id] ?? model.partialDownloadedBytes ?? 0;
  }

  function modelActionLabel(model: HuggingFaceVisionModel) {
    if (downloadingModelId === model.id) return cancelingModelId === model.id ? '取消中' : '取消下载';
    if (model.downloaded && model.updateAvailable) return '更新';
    if (model.downloaded) return '已下载';
    if (resumableBytes(model) > 0) return '继续';
    return '下载';
  }

  function modelActionDisabled(model: HuggingFaceVisionModel) {
    if (downloadingModelId === model.id) return cancelingModelId === model.id;
    if (downloadingModelId) return true;
    return (model.downloaded && !model.updateAvailable) || !validDownloadUrl(model.downloadUrl);
  }

  function formatCompactNumber(value: number) {
    if (value >= 1_000_000) return `${(value / 1_000_000).toFixed(1)}M`;
    if (value >= 1_000) return `${(value / 1_000).toFixed(1)}k`;
    return `${value}`;
  }

  function formatModelDate(value: string) {
    if (!value || value === 'unknown') return '未知';
    const date = new Date(value);
    if (Number.isNaN(date.getTime())) return '未知';
    return date.toISOString().slice(0, 10);
  }

  async function loadHuggingFaceCatalog(refresh = false) {
    const offset = refresh ? 0 : modelCatalog.length;
    const taskId = refresh ? 'hf-catalog-refresh' : 'hf-catalog-load';
    if (offset === 0) {
      isCatalogRefreshing = true;
      shell.startTask({
        id: taskId,
        label: refresh ? '刷新模型目录' : '加载模型目录',
        detail: '读取 Hugging Face 缓存',
        progress: null
      });
    } else {
      isLoadingMoreModels = true;
    }

    try {
      const page = await catalogRequestWithTimeout({ offset, limit: 20, refresh });
      if (page?.models.length) {
        modelCatalog = offset === 0 ? page.models : [...modelCatalog, ...page.models];
        modelCatalogTotal = page.total;
        modelCatalogHasMore = page.hasMore;
        modelCatalogCacheDate = page.cacheDate;
        modelCatalogStatus = 'ready';
        if (offset === 0) shell.finishTask(taskId, refresh ? '缓存已刷新' : '缓存已读取');
      } else if (offset === 0) {
        modelCatalog = [];
        modelCatalogTotal = 0;
        modelCatalogHasMore = false;
        modelCatalogCacheDate = '';
        modelCatalogStatus = 'fallback';
        shell.finishTask(taskId, '使用本地模型列表');
      }
    } finally {
      isCatalogRefreshing = false;
      isLoadingMoreModels = false;
    }
  }

  function catalogRequestWithTimeout(request: { offset: number; limit: number; refresh: boolean }) {
    return Promise.race([
      tryListHuggingFaceVisionModels(request),
      new Promise<null>((resolve) => {
        window.setTimeout(() => resolve(null), request.refresh ? 30000 : 3000);
      })
    ]);
  }

  async function loadMoreHuggingFaceModels() {
    if (!modelCatalogHasMore || isLoadingMoreModels || isCatalogRefreshing) return;
    await loadHuggingFaceCatalog(false);
  }

  function resetProviderDraft() {
    newProviderName = '';
    newProviderBaseUrl = 'http://localhost:11434/v1';
    newProviderModel = '';
    newProviderApiKey = '';
  }

  function validDownloadUrl(value: string) {
    return value.trim().startsWith('https://') || value.trim().startsWith('http://');
  }

  function canDownload(model: LocalModelConfig) {
    return validDownloadUrl(model.downloadUrl) && Boolean(model.fileName.trim()) && !downloadingModelId;
  }

  async function cancelActiveModelDownload(modelId: string) {
    if (downloadingModelId !== modelId || cancelingModelId) return;
    cancelingModelId = modelId;
    downloadMessage = '正在取消下载...';
    shell.updateTask(`model-download-${modelId}`, {
      detail: '正在取消下载',
      progress: progressForModel(modelId)?.percent ?? null
    });

    const cancelled = await tryCancelModelDownload(modelId);
    if (!cancelled) {
      cancelingModelId = '';
      downloadMessage = '取消失败，请稍后重试。';
      shell.updateTask(`model-download-${modelId}`, {
        detail: progressForModel(modelId) ? progressDetail(progressForModel(modelId)!) : '仍在下载',
        progress: progressForModel(modelId)?.percent ?? null
      });
    }
  }

  async function withDownloadProgress(
    taskId: string,
    modelId: string,
    initialBytes: number,
    run: () => Promise<DownloadModelSummary | null>
  ) {
    downloadProgress = {
      ...downloadProgress,
      [modelId]: { modelId, downloadedBytes: initialBytes, totalBytes: null, percent: initialBytes ? null : 0 }
    };

    let unlisten = () => {};
    try {
      unlisten = await listenModelDownloadProgress((progress) => {
        if (progress.modelId !== modelId) return;
        downloadProgress = { ...downloadProgress, [modelId]: progress };
        const detail = progressDetail(progress);
        const percentText = progress.percent === null ? '' : `${progress.percent}% · `;
        downloadMessage = `正在下载 ${percentText}${detail}`;
        shell.updateTask(taskId, {
          detail,
          progress: progress.percent
        });
      });
      return await run();
    } finally {
      unlisten();
    }
  }

  function buildConfig(): AppConfig {
    return {
      providerId: legacyProviderId(activeProvider),
      activeModelId: resolvedActiveModelId(),
      activeModelProviderId: activeProvider,
      activeLocalModelId,
      localModels: cloneLocalModels(localModels),
      thirdPartyProviders: cloneProviders(thirdPartyProviders),
      blurThreshold,
      exposureTolerance,
      cullLine,
      vlmThreads,
      arenaTarget,
      autoGroup,
      gpuMetal,
      shortcuts: shortcuts.map((shortcut) => ({ id: shortcut.id, keys: [...shortcut.keys] }))
    };
  }

  function markDirty(label: string) {
    if (!dirtyFields.includes(label)) dirtyFields.push(label);
  }

  function selectProvider(provider: string) {
    activeProvider = provider;
    markDirty(`供应商 → ${providerName(provider)}`);
  }

  function activateLocalModel(modelId: string) {
    activeProvider = 'llama.cpp';
    activeLocalModelId = modelId;
    markDirty(`本地模型 → ${localModels.find((model) => model.id === modelId)?.name ?? modelId}`);
  }

  async function downloadLocalModel(model: LocalModelConfig) {
    if (!canDownload(model)) {
      downloadMessage = '当前模型没有可用下载源，或文件名无效。';
      return;
    }
    const taskId = `model-download-${model.id}`;
    downloadingModelId = model.id;
    cancelingModelId = '';
    downloadMessage = `正在下载 ${model.name}...`;
    shell.startTask({
      id: taskId,
      label: '模型下载',
      detail: model.name,
      progress: 0
    });
    try {
      const summary = await withDownloadProgress(taskId, model.id, 0, () =>
        tryDownloadModel({
          modelId: model.id,
          fileName: model.fileName,
          downloadUrl: model.downloadUrl
        })
      );
      if (summary) {
        localModels = localModels.map((item) =>
          item.id === model.id ? { ...item, downloaded: true, localPath: summary.path } : item
        );
        activeProvider = 'llama.cpp';
        activeLocalModelId = model.id;
        downloadMessage = `已下载 ${formatBytes(summary.bytes)} 到 ${summary.path}`;
        forgetResumableDownload(model.id);
        markDirty(`下载模型 → ${model.name}`);
        shell.finishTask(taskId, '下载完成');
      } else if (cancelingModelId === model.id) {
        rememberResumableDownload(model.id);
        downloadMessage = `已取消 ${model.name} 下载`;
        shell.cancelTask(taskId, '已取消');
      } else {
        rememberResumableDownload(model.id);
        downloadMessage = '下载失败，请确认地址可访问、磁盘空间充足，并查看终端日志。';
        shell.failTask(taskId, '下载失败');
      }
    } finally {
      removeDownloadProgress(model.id);
      downloadingModelId = '';
      cancelingModelId = '';
    }
  }

  async function downloadCatalogModel(model: HuggingFaceVisionModel) {
    if (downloadingModelId && downloadingModelId !== model.id) return;
    if (modelActionDisabled(model) && !(model.downloaded && model.updateAvailable)) return;
    const taskId = `model-download-${model.id}`;
    const partialBytes = resumableBytes(model);
    downloadingModelId = model.id;
    cancelingModelId = '';
    downloadMessage = `${partialBytes ? '正在继续下载' : model.updateAvailable ? '正在更新' : '正在下载'} ${model.name}...`;
    shell.startTask({
      id: taskId,
      label: model.updateAvailable ? '模型更新' : '模型下载',
      detail: model.name,
      progress: 0
    });
    try {
      const summary = await withDownloadProgress(taskId, model.id, partialBytes, () =>
        tryDownloadModel({
          modelId: model.id,
          fileName: model.fileName,
          downloadUrl: model.downloadUrl
        })
      );
      if (summary) {
        const downloadedModel = {
          ...model,
          downloaded: true,
          updateAvailable: false,
          localPath: summary.path,
          partialDownloadedBytes: 0,
          partialPath: null
        };
        modelCatalog = modelCatalog.map((item) => (item.id === model.id ? downloadedModel : item));
        localModels = upsertLocalModelFromCatalog(downloadedModel, summary.path);
        activeProvider = 'llama.cpp';
        activeLocalModelId = model.id;
        downloadMessage = `已${model.updateAvailable ? '更新' : '下载'} ${formatBytes(summary.bytes)} 到 ${summary.path}`;
        forgetResumableDownload(model.id);
        markDirty(`${model.updateAvailable ? '更新' : '下载'}模型 → ${model.name}`);
        shell.finishTask(taskId, model.updateAvailable ? '更新完成' : '下载完成');
      } else if (cancelingModelId === model.id) {
        rememberResumableDownload(model.id);
        downloadMessage = `已取消 ${model.name} 下载`;
        shell.cancelTask(taskId, '已取消');
      } else {
        rememberResumableDownload(model.id);
        downloadMessage = '下载失败，请确认 Hugging Face 可访问、磁盘空间充足，并查看终端日志。';
        shell.failTask(taskId, '下载失败');
      }
    } finally {
      removeDownloadProgress(model.id);
      downloadingModelId = '';
      cancelingModelId = '';
    }
  }

  function upsertLocalModelFromCatalog(model: HuggingFaceVisionModel, localPath: string) {
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

  function addProvider() {
    if (!newProviderName.trim() || !newProviderBaseUrl.trim()) {
      configError = '添加供应商需要填写名称和 Base URL。';
      return;
    }
    const id = uniqueId(newProviderName, thirdPartyProviders.map((provider) => provider.id), 'provider');
    thirdPartyProviders = [
      ...thirdPartyProviders,
      {
        id,
        name: newProviderName.trim(),
        baseUrl: newProviderBaseUrl.trim(),
        apiKey: newProviderApiKey,
        model: newProviderModel.trim(),
        kind: 'openai-compatible',
        enabled: true
      }
    ];
    activeProvider = id;
    configError = '';
    showProviderDraft = false;
    resetProviderDraft();
    markDirty('添加第三方供应商');
  }

  function removeProvider(providerId: string) {
    thirdPartyProviders = thirdPartyProviders.filter((provider) => provider.id !== providerId);
    if (activeProvider === providerId) activeProvider = 'llama.cpp';
    markDirty('删除第三方供应商');
  }

  function updateProvider(
    providerId: string,
    field: 'name' | 'baseUrl' | 'apiKey' | 'model' | 'kind',
    value: string
  ) {
    thirdPartyProviders = thirdPartyProviders.map((provider) =>
      provider.id === providerId ? { ...provider, [field]: value } : provider
    );
    markDirty(`供应商 ${field}`);
  }

  function toggleProvider(providerId: string) {
    thirdPartyProviders = thirdPartyProviders.map((provider) =>
      provider.id === providerId ? { ...provider, enabled: !provider.enabled } : provider
    );
    markDirty('供应商启用状态');
  }

  function updateNumber(field: 'blur' | 'exposure' | 'cull' | 'threads', value: number) {
    if (field === 'blur') {
      blurThreshold = value;
      markDirty(`模糊阈值 → ${value}`);
    }
    if (field === 'exposure') {
      exposureTolerance = value;
      markDirty(`曝光宽容度 → ${value.toFixed(3)}`);
    }
    if (field === 'cull') {
      cullLine = value;
      markDirty(`淘汰线 → ${value}`);
    }
    if (field === 'threads') {
      vlmThreads = value;
      markDirty(`并发线程 → ${value}`);
    }
  }

  function remapShortcut(event: KeyboardEvent, shortcut: Shortcut) {
    if (editingShortcut !== shortcut.id) return;
    if (event.key === 'Escape') {
      editingShortcut = null;
      return;
    }
    if (event.key === 'Tab') return;

    event.preventDefault();
    const display = normalizeKey(event.key);
    shortcut.keys = [display];
    editingShortcut = null;
    markDirty(`快捷键 ${shortcut.action} → ${display}`);
  }

  function normalizeKey(key: string) {
    if (key === ' ') return 'Space';
    if (key === 'ArrowLeft') return 'Left';
    if (key === 'ArrowRight') return 'Right';
    if (key === 'ArrowUp') return 'Up';
    if (key === 'ArrowDown') return 'Down';
    if (key === 'Meta') return 'Cmd';
    return key.length === 1 ? key.toUpperCase() : key;
  }

  async function saveChanges() {
    if (isSavingConfig) return;
    isSavingConfig = true;
    configError = '';
    const envelope = await trySaveAppConfig(buildConfig());
    if (envelope) {
      configPath = envelope.configPath;
      appDataDir = envelope.appDataDir;
      applyConfig(envelope.config);
    } else {
      configError = '保存失败，请确认当前运行在 Tauri 桌面环境。';
    }
    isSavingConfig = false;
  }
</script>

{#snippet settingsSidebar()}
  <p class="nav-eyebrow">设置分组</p>
  <div class="nav-section">
    <a class="nav-item active" href="#provider">推理引擎 · Provider</a>
    <a class="nav-item" href="#models">VLM 模型管理</a>
    <a class="nav-item" href="#modes">模式参数</a>
    <a class="nav-item" href="#shortcuts">键盘快捷键</a>
    <a class="nav-item" href="#about">数据与关于</a>
  </div>
{/snippet}

<main class="app-main scroll">
    <div class="settings-wrap">
      <header class="page-head">
        <p class="page-num">设置</p>
        <h1 class="page-title">推理引擎 · 模型 · 模式 · 快捷键</h1>
        <p class="page-lede">
          {isLoadingConfig ? '正在读取本地配置...' : `所有改动先进入本地草稿；保存后写入 ${configPath}`}
        </p>
      </header>

      <section id="provider" class="section">
        <div class="settings-head">
          <h2>模型供应商</h2>
          <span>{activeProvider} / {activeModelId}</span>
        </div>

        <div class="provider-workspace">
          <aside class="provider-directory" aria-label="模型供应商列表">
            <div class="directory-search">
              <input bind:value={providerSearch} placeholder="搜索供应商、模型或地址" aria-label="搜索模型供应商" />
            </div>

            <button
              type="button"
              class={['provider-list-item', activeProvider === 'llama.cpp' && 'active'].filter(Boolean).join(' ')}
              aria-pressed={activeProvider === 'llama.cpp'}
              onclick={() => selectProvider('llama.cpp')}
            >
              <span class="provider-avatar local">ll</span>
              <span class="provider-list-copy">
                <strong>本地模型</strong>
                <small>{providerModelSummary('llama.cpp')}</small>
              </span>
              <span class="provider-pill local">LOCAL</span>
            </button>

            <div class="directory-divider">
              <span>第三方供应商</span>
              <span>{filteredThirdPartyProviders.length}</span>
            </div>

            {#each filteredThirdPartyProviders as provider (provider.id)}
              <button
                type="button"
                class={['provider-list-item', activeProvider === provider.id && 'active'].filter(Boolean).join(' ')}
                aria-pressed={activeProvider === provider.id}
                onclick={() => selectProvider(provider.id)}
              >
                <span class="provider-avatar">{providerInitials(provider)}</span>
                <span class="provider-list-copy">
                  <strong>{provider.name || provider.id}</strong>
                  <small>{selectedProviderSummary(provider)}</small>
                </span>
                <span class={['provider-pill', provider.enabled && provider.model && 'on'].filter(Boolean).join(' ')}>
                  {providerStatusLabel(provider)}
                </span>
              </button>
            {/each}

            {#if showProviderDraft}
              <div class="provider-draft">
                <label>
                  <span>名称</span>
                  <input bind:value={newProviderName} placeholder="例如 SiliconFlow" />
                </label>
                <label>
                  <span>API Host</span>
                  <input bind:value={newProviderBaseUrl} placeholder="https://api.example.com/v1" />
                </label>
                <label>
                  <span>默认模型</span>
                  <input bind:value={newProviderModel} placeholder="provider/model-name" />
                </label>
                <label>
                  <span>API Key</span>
                  <input type="password" bind:value={newProviderApiKey} placeholder="可选" />
                </label>
                <div class="draft-actions">
                  <button type="button" class="btn btn-secondary" onclick={() => { showProviderDraft = false; resetProviderDraft(); }}>取消</button>
                  <button type="button" class="btn btn-primary" onclick={addProvider}>添加</button>
                </div>
              </div>
            {:else}
              <button type="button" class="provider-add-button" onclick={() => (showProviderDraft = true)}>
                <span>+</span>
                添加供应商
              </button>
            {/if}
          </aside>

          <div class="provider-panel">
            <div class="provider-panel-head">
              <div>
                <h3>{activeProviderName}</h3>
                <p>{activeProvider === 'llama.cpp' ? '由 llama.cpp 驱动 · 从 Hugging Face 下载 GGUF 视觉模型' : activeProviderConfig ? providerStatusText(activeProviderConfig) : '未选择供应商'}</p>
              </div>
              {#if activeProviderConfig}
                <div class="provider-head-actions">
                  <button
                    type="button"
                    class={['toggle', activeProviderConfig.enabled && 'on'].filter(Boolean).join(' ')}
                    aria-label="切换供应商启用状态"
                    aria-pressed={activeProviderConfig.enabled}
                    onclick={() => toggleProvider(activeProviderConfig.id)}
                  ></button>
                  <button type="button" class="text-action danger" onclick={() => removeProvider(activeProviderConfig.id)}>删除</button>
                </div>
              {/if}
            </div>

            {#if activeProvider === 'llama.cpp'}
              <div class="local-model-summary">
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
              </div>

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
                  <input bind:value={modelCatalogSearch} placeholder="搜索 Hugging Face 视觉模型" aria-label="搜索 Hugging Face 视觉模型" />
                  <button type="button" class="model-action" disabled={isCatalogRefreshing} onclick={() => void loadHuggingFaceCatalog(true)}>
                    {isCatalogRefreshing ? '刷新中' : '刷新缓存'}
                  </button>
                </div>

                <div class="hf-model-table" role="list" aria-label="Hugging Face 视觉模型">
                  {#each visibleCatalogModels as model (model.id)}
                    {@const progress = progressForModel(model.id)}
                    {@const partialBytes = resumableBytes(model)}
                    <div class={['hf-model-row', model.id === activeLocalModelId && 'active'].filter(Boolean).join(' ')} role="listitem">
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
                        <button
                          type="button"
                          class={[
                            'model-action',
                            model.downloaded && !model.updateAvailable && 'done',
                            model.updateAvailable && 'update',
                            downloadingModelId === model.id && 'cancel'
                          ].filter(Boolean).join(' ')}
                          disabled={modelActionDisabled(model)}
                          onclick={() =>
                            void (downloadingModelId === model.id ? cancelActiveModelDownload(model.id) : downloadCatalogModel(model))}
                        >
                          {progressButtonLabel(model, progress)}
                        </button>
                        {#if progress}
                          <span class="download-progress-track" aria-label="下载进度">
                            <span style={`width: ${progress.percent ?? 0}%`}></span>
                          </span>
                        {/if}
                      </span>
                    </div>
                  {:else}
                    <div class="hf-model-empty">没有匹配的视觉模型。</div>
                  {/each}
                </div>

                {#if modelCatalogHasMore && !modelCatalogSearch.trim()}
                  <button type="button" class="load-more-models" disabled={isLoadingMoreModels} onclick={() => void loadMoreHuggingFaceModels()}>
                    {isLoadingMoreModels ? '加载中' : `加载更多 · ${modelCatalog.length} / ${modelCatalogTotal}`}
                  </button>
                {/if}

                {#if downloadMessage}
                  <p class="download-message">{downloadMessage}</p>
                {/if}
              </div>
            {:else if activeProviderConfig}
              <div class="provider-fields remote-provider-fields">
                <label>
                  <span>供应商 ID</span>
                  <input value={activeProviderConfig.id} readonly />
                </label>
                <label>
                  <span>类型</span>
                  <input value="@ai-sdk/openai-compatible" readonly />
                </label>
                <label>
                  <span>名称</span>
                  <input value={activeProviderConfig.name} oninput={(event) => updateProvider(activeProviderConfig.id, 'name', event.currentTarget.value)} />
                </label>
                <label>
                  <span>Base URL</span>
                  <input value={activeProviderConfig.baseUrl} placeholder="http://localhost:11434/v1" oninput={(event) => updateProvider(activeProviderConfig.id, 'baseUrl', event.currentTarget.value)} />
                </label>
                <label>
                  <span>API Key</span>
                  <input type="password" value={activeProviderConfig.apiKey} placeholder="本地服务可留空" oninput={(event) => updateProvider(activeProviderConfig.id, 'apiKey', event.currentTarget.value)} />
                </label>
                <label>
                  <span>Model ID</span>
                  <input value={activeProviderConfig.model} placeholder="llava:latest / gpt-4o-mini" oninput={(event) => updateProvider(activeProviderConfig.id, 'model', event.currentTarget.value)} />
                </label>
              </div>

              <div id="models" class="remote-models">
                <div class="model-library-title">
                  <h4>默认模型</h4>
                  <span>{activeProviderConfig.enabled ? '启用' : '停用'}</span>
                </div>
                <div class="remote-model-row">
                  <span class="model-icon">{(activeProviderConfig.model || '?').slice(0, 1).toUpperCase()}</span>
                  <div>
                    <strong>{activeProviderConfig.model || '尚未填写模型 ID'}</strong>
                    <small>{activeProviderConfig.baseUrl || '尚未配置 API Host'}</small>
                  </div>
                  <button type="button" class="model-action" onclick={() => selectProvider(activeProviderConfig.id)}>使用</button>
                </div>
              </div>
            {:else}
              <div class="empty-provider">
                <strong>未选择可用供应商</strong>
                <span>请选择 llama.cpp 或添加一个 OpenAI-compatible provider。</span>
              </div>
            {/if}

          </div>
        </div>
      </section>

      <section id="modes" class="section">
        <div class="settings-head">
          <h2>模式参数</h2>
          <span>阈值决定自动淘汰的激进程度</span>
        </div>
        <p class="section-copy">快速模式会读取模糊、曝光、淘汰线和连拍分组参数；竞技场与 VLM 线程参数会先保存为后续能力配置。</p>

        <div class="form-row">
          <span><strong>模糊检测阈值</strong><small>Laplacian 方差低于此值判定模糊</small></span>
          <label class="range-control">
            <input type="range" min="0" max="300" step="5" value={blurThreshold} oninput={(event) => updateNumber('blur', Number(event.currentTarget.value))} />
            <span>{blurThreshold}.0</span>
          </label>
          <span class="form-value">默认 100</span>
        </div>

        <div class="form-row">
          <span><strong>曝光宽容度</strong><small>直方图两端裁切比例</small></span>
          <label class="range-control">
            <input type="range" min="0" max="0.1" step="0.001" value={exposureTolerance} oninput={(event) => updateNumber('exposure', Number(event.currentTarget.value))} />
            <span>{exposureTolerance.toFixed(3)}</span>
          </label>
          <span class="form-value">0.018</span>
        </div>

        <div class="form-row">
          <span><strong>质量分淘汰线</strong><small>低于此值自动进入淘汰池</small></span>
          <label class="range-control">
            <input type="range" min="0" max="100" step="1" value={cullLine} oninput={(event) => updateNumber('cull', Number(event.currentTarget.value))} />
            <span>{cullLine} / 100</span>
          </label>
          <span class="form-value">自动模式</span>
        </div>

        <div class="form-row">
          <span><strong>竞技场目标保留</strong><small>循环 PK 直到剩余此比例</small></span>
          <select bind:value={arenaTarget} onchange={() => markDirty(`竞技场目标 → ${arenaTarget}`)}>
            <option>10%</option>
            <option>20%</option>
            <option>30%</option>
            <option>50%</option>
          </select>
          <span class="form-value">约 250 张</span>
        </div>

        <div class="form-row">
          <span><strong>并发推理线程</strong><small>Semaphore 同时进行的 VLM 调用数</small></span>
          <label class="range-control">
            <input type="range" min="1" max="8" step="1" value={vlmThreads} oninput={(event) => updateNumber('threads', Number(event.currentTarget.value))} />
            <span>{vlmThreads}</span>
          </label>
          <span class="form-value">最大 8</span>
        </div>

        <div class="form-row">
          <span><strong>连拍自动分组</strong><small>基于 pHash 相似度自动聚合连拍</small></span>
          <button
            type="button"
            class={['toggle', autoGroup && 'on'].filter(Boolean).join(' ')}
            aria-label="切换连拍自动分组"
            aria-pressed={autoGroup}
            onclick={() => { autoGroup = !autoGroup; markDirty('连拍自动分组'); }}
          ></button>
          <span class="form-value">{autoGroup ? '已开启' : '已关闭'}</span>
        </div>

        <div class="form-row">
          <span><strong>GPU 加速 · Metal</strong><small>使用 Apple GPU 进行推理加速</small></span>
          <button
            type="button"
            class={['toggle', gpuMetal && 'on'].filter(Boolean).join(' ')}
            aria-label="切换 Metal GPU 加速"
            aria-pressed={gpuMetal}
            onclick={() => { gpuMetal = !gpuMetal; markDirty('GPU 加速'); }}
          ></button>
          <span class="form-value">{gpuMetal ? '已开启' : '已关闭'}</span>
        </div>
      </section>

      <section id="shortcuts" class="section">
        <div class="settings-head">
          <h2>键盘快捷键</h2>
          <span>点击按键即可重映射 · 文本框中自动暂停</span>
        </div>
        <p class="section-copy">摄影师的肌肉记忆比工具默认值重要。点击任意快捷键单元即可重映射，保存后挑选页会按本地配置执行。</p>
        <table class="shortcuts-table">
          <thead>
            <tr><th>动作</th><th>场景</th><th>快捷键</th></tr>
          </thead>
          <tbody>
            {#each shortcuts as shortcut (shortcut.id)}
              <tr>
                <td>{shortcut.action}</td>
                <td>{shortcut.scenario}</td>
                <td class="keys">
                  <button
                    type="button"
                    class={['kbd-cell', editingShortcut === shortcut.id && 'editing'].filter(Boolean).join(' ')}
                    onkeydown={(event) => remapShortcut(event, shortcut)}
                    onclick={() => (editingShortcut = shortcut.id)}
                  >
                    {#each shortcut.keys as key (`${shortcut.id}-${key}`)}
                      <span class="kbd">{key}</span>
                    {/each}
                    <span class="hint">{editingShortcut === shortcut.id ? '按键...' : '编辑'}</span>
                  </button>
                </td>
              </tr>
            {/each}
          </tbody>
        </table>
      </section>

      <section id="about" class="section">
        <div class="settings-head">
          <h2>关于</h2>
          <span>本地离线 · 商业可交付目标</span>
        </div>
        <div class="colophon">
          <p>Cullify 0.1.0 是面向摄影师的本地 AI 选片工具。当前版本已接入 Rust 扫描器、SQLite 持久化、快速质量评分、手动决策保存和 JSON / CSV / ZIP 导出；专家 Provider 推理会在后续版本继续增强。</p>
          <div>
            <span><b>桌面框架</b><em>Tauri 2</em></span>
            <span><b>前端</b><em>Svelte 5</em></span>
            <span><b>后端语言</b><em>Rust · Edition 2024</em></span>
            <span><b>数据目录</b><em>{appDataDir}</em></span>
          </div>
        </div>
      </section>
    </div>

    <div class="save-bar">
      <div>
        <span class="save-status">
          {configError || (isSavingConfig ? '正在保存配置...' : dirtyCount ? `${dirtyCount} 项未保存改动` : '配置已保存')}
        </span>
        {#if dirtyCount}
          <div class="save-diff">
            {#each dirtyFields.slice(-4) as field (field)}
              <span>{field}</span>
            {/each}
          </div>
        {/if}
      </div>
      <div class="save-actions">
        <button class="btn btn-secondary" type="button" onclick={() => void loadSavedConfig()} disabled={isSavingConfig}>放弃改动</button>
        <button class="btn btn-primary" type="button" onclick={() => void saveChanges()} disabled={isSavingConfig}>
          {isSavingConfig ? '保存中' : '保存到 config.toml'}
        </button>
      </div>
    </div>
  </main>

<style>
  .settings-wrap {
    max-width: 1180px;
    padding: 36px 48px 64px;
  }

  .page-head {
    margin-bottom: 36px;
  }

  .page-num {
    margin: 0 0 4px;
    color: var(--accent);
    font-family: var(--font-display);
    font-size: 13px;
  }

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

  .settings-head span,
  .section-copy {
    color: var(--muted);
    font-size: 12px;
  }

  .section-copy {
    max-width: 72ch;
    margin: -8px 0 18px;
    line-height: 1.65;
  }

  .provider-workspace {
    display: grid;
    grid-template-columns: 300px minmax(0, 1fr);
    min-height: 560px;
    overflow: hidden;
    border: 1px solid #e7e7e2;
    border-radius: 8px;
    background: #fff;
    box-shadow: var(--shadow-soft);
  }

  .provider-directory {
    display: grid;
    grid-auto-rows: max-content;
    gap: 8px;
    border-right: 1px solid #ecece8;
    background: #fbfbfa;
    padding: 14px 12px;
  }

  .directory-search input {
    width: 100%;
    border-radius: 999px;
    background: #fff;
    padding: 10px 14px;
  }

  .directory-divider,
  .model-library-title,
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

  .provider-list-item {
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

  .provider-list-item:hover,
  .provider-list-item.active {
    border-color: #d7e9df;
    background: #fff;
  }

  .provider-list-item.active {
    box-shadow: 0 8px 18px rgba(20, 20, 19, 0.055);
  }

  .provider-avatar,
  .model-icon {
    display: grid;
    place-items: center;
    width: 34px;
    height: 34px;
    flex: 0 0 auto;
    border-radius: 50%;
    background: #edf4ff;
    color: #295b92;
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 700;
    text-transform: uppercase;
  }

  .provider-avatar.local {
    background: #eef8f2;
    color: #167246;
  }

  .provider-list-copy,
  .model-row-copy {
    min-width: 0;
  }

  .provider-list-copy strong,
  .model-row-copy strong,
  .remote-model-row strong {
    display: block;
    overflow: hidden;
    color: var(--fg);
    font-size: 14px;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .provider-list-copy small,
  .model-row-copy small,
  .remote-model-row small {
    display: block;
    overflow: hidden;
    color: #8b8a84;
    font-family: var(--font-mono);
    font-size: 10px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .provider-pill,
  .model-state {
    border: 1px solid #e4e2db;
    border-radius: 999px;
    background: #f6f5f0;
    color: #7a7770;
    font-family: var(--font-mono);
    font-size: 10px;
    padding: 3px 8px;
  }

  .provider-pill.on,
  .provider-pill.local,
  .model-state.ready {
    border-color: #bce7cf;
    background: #effaf4;
    color: #167246;
  }

  .provider-add-button {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
    min-height: 40px;
    border: 1px solid #e1dfd6;
    border-radius: 999px;
    background: #fff;
    color: var(--fg-2);
    font-weight: 700;
  }

  .provider-add-button span {
    font-size: 20px;
    line-height: 1;
  }

  .provider-workspace button:focus {
    outline: none;
  }

  .provider-workspace button:focus-visible,
  .provider-workspace input:focus-visible {
    outline: 2px solid rgba(16, 185, 129, 0.28);
    outline-offset: 2px;
  }

  .provider-draft {
    display: grid;
    gap: 10px;
    border: 1px solid #e7e7e2;
    border-radius: 8px;
    background: #fff;
    padding: 12px;
  }

  .provider-draft label,
  .provider-fields label {
    display: grid;
    gap: 6px;
    min-width: 0;
  }

  .provider-draft span,
  .provider-fields span,
  .field-caption {
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
    background: #fff;
    padding: 22px 26px 28px;
  }

  .provider-panel-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    min-height: 52px;
    border-bottom: 1px solid #ecece8;
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
    color: #8b8a84;
    font-size: 12px;
    overflow-wrap: anywhere;
  }

  .provider-fields {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: 14px 16px;
  }

  .remote-provider-fields {
    border: 1px solid #efeee9;
    border-radius: 8px;
    background: #fcfcfb;
    padding: 16px;
  }

  .provider-fields.single {
    grid-template-columns: 1fr;
    gap: 8px;
  }

  .local-model-summary {
    display: grid;
    grid-template-columns: 1.45fr 0.8fr 0.65fr;
    gap: 1px;
    overflow: hidden;
    border: 1px solid #ecece8;
    border-radius: 8px;
    background: #ecece8;
  }

  .local-model-summary div {
    display: grid;
    gap: 5px;
    min-width: 0;
    background: #fbfbfa;
    padding: 12px 14px;
  }

  .local-model-summary span {
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 10px;
    text-transform: uppercase;
  }

  .local-model-summary strong {
    overflow: hidden;
    color: var(--fg-2);
    font-size: 12px;
    font-weight: 700;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  input[readonly] {
    color: var(--meta);
    cursor: default;
  }

  .model-library,
  .remote-models {
    display: grid;
    gap: 12px;
  }

  .model-library-title h4 {
    margin: 0;
    color: var(--fg);
    font-size: 16px;
    font-weight: 800;
  }

  .model-library-title span {
    color: #8b8a84;
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .model-catalog-toolbar {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px;
  }

  .model-catalog-toolbar input {
    width: 100%;
    border-radius: 999px;
    background: #fff;
    padding: 10px 14px;
  }

  .hf-model-table {
    display: grid;
    gap: 8px;
  }

  .hf-model-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) 140px;
    align-items: center;
    gap: 18px;
    min-height: 82px;
    border: 1px solid #ecece8;
    border-radius: 8px;
    background: #fff;
    padding: 12px 14px;
  }

  .hf-model-row:hover,
  .hf-model-row.active {
    border-color: #d9eadf;
    background: #fcfffd;
  }

  .hf-model-row.active {
    box-shadow: inset 3px 0 0 #10b981, 0 8px 18px rgba(20, 20, 19, 0.045);
  }

  .hf-model-name {
    display: grid;
    grid-template-columns: 34px minmax(0, 1fr);
    align-items: center;
    gap: 12px;
    min-width: 0;
  }

  .hf-model-name strong {
    display: block;
    overflow: hidden;
    color: var(--fg);
    font-size: 14px;
    font-weight: 800;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hf-model-copy {
    min-width: 0;
  }

  .hf-model-name small,
  .hf-model-copy em,
  .hf-model-meta small {
    display: block;
    overflow: hidden;
    color: #8b8a84;
    font-family: var(--font-mono);
    font-size: 10px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .hf-model-copy em {
    margin-top: 3px;
    color: #a09d96;
    font-style: normal;
  }

  .hf-model-meta {
    min-width: 0;
    color: var(--fg-2);
    font-family: var(--font-mono);
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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

  .download-progress-track {
    width: 100%;
    height: 4px;
    overflow: hidden;
    border-radius: 999px;
    background: #ebe8df;
  }

  .download-progress-track span {
    display: block;
    height: 100%;
    border-radius: inherit;
    background: #16b978;
    transition: width 160ms ease;
  }

  .hf-model-empty {
    color: var(--meta);
    padding: 18px;
  }

  .load-more-models {
    width: 100%;
    min-height: 38px;
    border: 1px solid #e1dfd6;
    border-radius: 999px;
    background: #fff;
    color: var(--fg-2);
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .load-more-models:disabled {
    color: var(--meta);
    cursor: progress;
  }

  .model-group {
    border: 1px solid #ecece8;
    border-radius: 8px;
    background: #fff;
  }

  .model-group-head {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
    min-height: 44px;
    border-bottom: 1px solid #ecece8;
    background: #f6f6f4;
    padding: 0 16px;
    text-align: left;
  }

  .model-group-head span {
    color: #5f5d57;
    font-size: 15px;
  }

  .model-group-head strong {
    color: var(--fg-2);
    font-size: 14px;
    font-weight: 800;
  }

  .model-row-button,
  .remote-model-row {
    display: grid;
    grid-template-columns: 34px minmax(0, 1fr) auto;
    align-items: center;
    gap: 12px;
    width: 100%;
    min-height: 68px;
    border: 1px solid #ecece8;
    border-radius: 8px;
    background: #fff;
    padding: 12px 18px;
    text-align: left;
  }

  .model-row-button:hover,
  .model-row-button.active {
    background: #fbfcff;
  }

  .model-row-button.active {
    box-shadow: inset 3px 0 0 #10b981;
  }

  .empty-provider {
    display: grid;
    gap: 14px;
    border: 1px solid #e1eee7;
    border-radius: 8px;
    background: #fbfffd;
    padding: 16px;
  }

  input,
  select {
    min-width: 0;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--fg);
    font: inherit;
    padding: 7px 10px;
  }

  input::placeholder {
    color: var(--meta);
  }

  .text-action {
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .text-action.danger {
    color: var(--danger);
  }

  .model-action,
  .form-value {
    color: var(--fg-2);
    font-family: var(--font-mono);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    white-space: pre-line;
  }

  .model-action {
    border: 1px solid var(--border);
    border-radius: 4px;
    background: transparent;
    color: var(--accent);
    padding: 5px 10px;
    text-transform: uppercase;
  }

  .model-action:disabled {
    border-color: var(--border-soft);
    color: var(--meta);
    cursor: not-allowed;
  }

  .model-action.done {
    border-color: #bce7cf;
    background: #effaf4;
    color: #167246;
  }

  .model-action.update {
    border-color: #f0d292;
    background: #fff7df;
    color: #8a5a00;
  }

  .model-action.cancel {
    border-color: #f1b8b0;
    background: #fff1ef;
    color: var(--danger);
  }

  .download-message {
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 10px;
    margin: 10px 0 0;
    overflow-wrap: anywhere;
  }

  .form-row {
    display: grid;
    grid-template-columns: 210px 1fr 90px;
    align-items: center;
    gap: 24px;
    border-bottom: 1px solid var(--border-soft);
    padding: 14px 0;
  }

  .form-row strong {
    display: block;
    font-family: var(--font-display);
    font-size: 14px;
    font-weight: 500;
  }

  .form-row small {
    display: block;
    margin-top: 3px;
    color: var(--meta);
    font-size: 12px;
  }

  .range-control {
    display: grid;
    grid-template-columns: minmax(120px, 280px) 76px;
    align-items: center;
    gap: 12px;
  }

  input[type="range"] {
    width: 100%;
    accent-color: var(--accent);
  }

  .range-control span {
    color: var(--fg);
    font-family: var(--font-mono);
    font-size: 12px;
  }

  .toggle {
    position: relative;
    width: 36px;
    height: 20px;
    border-radius: 999px;
    background: var(--border);
  }

  .toggle::after {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: var(--surface);
    content: "";
    transition: transform 0.15s ease;
  }

  .toggle.on {
    background: var(--accent);
  }

  .toggle.on::after {
    transform: translateX(16px);
  }

  .shortcuts-table {
    width: 100%;
    border-collapse: collapse;
  }

  .shortcuts-table th,
  .shortcuts-table td {
    border-bottom: 1px solid var(--border-soft);
    padding: 10px 14px;
    text-align: left;
  }

  .shortcuts-table th {
    border-bottom-color: var(--border);
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 500;
    letter-spacing: 1.1px;
    text-transform: uppercase;
  }

  .shortcuts-table td:first-child {
    color: var(--fg);
    font-family: var(--font-display);
    font-weight: 500;
  }

  .shortcuts-table td:nth-child(2) {
    color: var(--muted);
    font-size: 12px;
  }

  .keys {
    text-align: right;
  }

  .kbd-cell {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    border-radius: 4px;
    background: transparent;
    padding: 4px 8px;
  }

  .kbd-cell:hover,
  .kbd-cell.editing {
    background: var(--tag-bg-soft);
  }

  .hint {
    margin-left: 6px;
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 9px;
    text-transform: uppercase;
  }

  .colophon {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    padding: 24px 28px;
  }

  .colophon p {
    margin: 0 0 16px;
    border-bottom: 1px solid var(--border-soft);
    color: var(--muted);
    line-height: 1.7;
    padding-bottom: 14px;
  }

  .colophon div {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0 24px;
  }

  .colophon span {
    display: flex;
    justify-content: space-between;
    border-bottom: 1px solid var(--border-soft);
    padding: 7px 0;
  }

  .colophon b,
  .colophon em {
    font-family: var(--font-mono);
    font-size: 11px;
    font-style: normal;
    font-weight: 500;
  }

  .colophon b {
    color: var(--meta);
    text-transform: uppercase;
  }

  .colophon em {
    color: var(--fg-2);
  }

  .save-bar {
    position: sticky;
    bottom: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 18px;
    border-top: 1px solid var(--border);
    background: var(--surface);
    margin: 0 -48px -64px;
    padding: 14px 48px;
  }

  .save-status {
    display: flex;
    align-items: center;
    gap: 8px;
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 11px;
  }

  .save-status::before {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--warn);
    content: "";
  }

  .save-diff {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    margin-top: 4px;
  }

  .save-diff span {
    border-radius: 2px;
    background: var(--tag-bg-faint);
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: 10px;
    padding: 1px 6px;
  }

  .save-actions {
    display: flex;
    gap: 8px;
  }

  @media (max-width: 1000px) {
    .provider-workspace {
      grid-template-columns: 1fr;
    }

    .provider-directory {
      border-right: 0;
      border-bottom: 1px solid var(--border);
    }

    .provider-fields {
      grid-template-columns: 1fr;
    }

    .local-model-summary {
      grid-template-columns: 1fr;
    }

    .form-row {
      grid-template-columns: 1fr;
      align-items: start;
    }

    .provider-panel-head,
    .remote-model-row,
    .model-row-button,
    .hf-model-row {
      flex-direction: column;
      align-items: stretch;
    }

    .remote-model-row,
    .model-row-button {
      grid-template-columns: 34px minmax(0, 1fr);
    }

    .remote-model-row .model-action,
    .model-row-button .model-state {
      grid-column: 2;
      justify-self: start;
    }

    .hf-model-row {
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

    .hf-model-meta {
      white-space: normal;
    }

    .model-catalog-toolbar {
      grid-template-columns: 1fr;
    }

    .save-bar,
    .settings-head {
      align-items: flex-start;
      flex-direction: column;
    }
  }

  @media (max-width: 760px) {
    .settings-wrap {
      padding: 24px 18px 96px;
    }

    .colophon div {
      grid-template-columns: 1fr;
    }
  }
</style>
