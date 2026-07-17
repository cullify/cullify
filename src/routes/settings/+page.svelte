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
  import { Button } from '$lib/components/ui/button';
  import * as Card from '$lib/components/ui/card';
  import ModeParameters from '$lib/settings/ModeParameters.svelte';
  import ProviderSettings from '$lib/settings/ProviderSettings.svelte';
  import ShortcutSettings from '$lib/settings/ShortcutSettings.svelte';
  import {
    catalogModelFromLocalModel,
    cloneLocalModels,
    cloneProviders,
    formatBytes,
    formatCompactNumber,
    legacyProviderId,
    normalizeProvider,
    uniqueId,
    upsertLocalModelFromCatalog,
    validDownloadUrl,
    type CatalogStatus,
    type NumberSettingField
  } from '$lib/settings/modelHelpers';
  import { getShellContext } from '$lib/shell.svelte';
  import RotateCcwIcon from '@lucide/svelte/icons/rotate-ccw';
  import SaveIcon from '@lucide/svelte/icons/save';
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
  let modelCatalogStatus = $state<CatalogStatus>('fallback');
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

  function providerName(providerId: string) {
    if (providerId === 'llama.cpp') return '本地模型';
    return thirdPartyProviders.find((provider) => provider.id === providerId)?.name ?? '未配置';
  }

  function resolvedActiveModelId() {
    if (activeProvider === 'llama.cpp') return activeLocalModelId;
    return thirdPartyProviders.find((provider) => provider.id === activeProvider)?.model || '';
  }

  function selectedLocalModelPath(model: LocalModelConfig) {
    return model.localPath || `${appDataDir}/models/${model.fileName}`;
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

  function providerModelSummary(providerId: string) {
    if (providerId === 'llama.cpp') return `llama.cpp 驱动 · ${activeLocalModel?.name ?? activeLocalModelId}`;
    return thirdPartyProviders.find((provider) => provider.id === providerId)?.model || '未指定模型';
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

  function setProviderDraftField(field: 'name' | 'baseUrl' | 'model' | 'apiKey', value: string) {
    if (field === 'name') newProviderName = value;
    if (field === 'baseUrl') newProviderBaseUrl = value;
    if (field === 'model') newProviderModel = value;
    if (field === 'apiKey') newProviderApiKey = value;
  }

  function cancelProviderDraft() {
    showProviderDraft = false;
    resetProviderDraft();
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
        localModels = upsertLocalModelFromCatalog(localModels, downloadedModel, summary.path);
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

  function updateNumber(field: NumberSettingField, value: number) {
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

  function setArenaTarget(value: string) {
    arenaTarget = value;
    markDirty(`竞技场目标 → ${arenaTarget}`);
  }

  function toggleAutoGroup() {
    autoGroup = !autoGroup;
    markDirty('连拍自动分组');
  }

  function toggleGpuMetal() {
    gpuMetal = !gpuMetal;
    markDirty('GPU 加速');
  }

  function setEditingShortcut(shortcutId: string) {
    editingShortcut = shortcutId;
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

      <ProviderSettings
        {activeProvider}
        {activeModelId}
        {activeProviderName}
        {activeProviderConfig}
        {activeLocalModelId}
        {appDataDir}
        {filteredThirdPartyProviders}
        {providerSearch}
        {showProviderDraft}
        providerDraft={{
          name: newProviderName,
          baseUrl: newProviderBaseUrl,
          model: newProviderModel,
          apiKey: newProviderApiKey
        }}
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
        {providerModelSummary}
        setProviderSearch={(value) => (providerSearch = value)}
        {setProviderDraftField}
        showDraft={() => (showProviderDraft = true)}
        cancelDraft={cancelProviderDraft}
        {addProvider}
        {removeProvider}
        {toggleProvider}
        {selectProvider}
        {updateProvider}
        {progressForModel}
        {progressButtonLabel}
        {modelStatusText}
        {resumableBytes}
        {modelActionDisabled}
        setModelCatalogSearch={(value) => (modelCatalogSearch = value)}
        refreshCatalog={() => void loadHuggingFaceCatalog(true)}
        loadMoreModels={() => void loadMoreHuggingFaceModels()}
        downloadCatalogModel={(model) => void downloadCatalogModel(model)}
        cancelActiveModelDownload={(modelId) => void cancelActiveModelDownload(modelId)}
      />

      <ModeParameters
        {blurThreshold}
        {exposureTolerance}
        {cullLine}
        {vlmThreads}
        {arenaTarget}
        {autoGroup}
        {gpuMetal}
        {updateNumber}
        {setArenaTarget}
        {toggleAutoGroup}
        {toggleGpuMetal}
      />

      <ShortcutSettings {shortcuts} {editingShortcut} {setEditingShortcut} {remapShortcut} />

      <section id="about" class="section">
        <div class="settings-head">
          <h2>关于</h2>
          <span>本地离线 · 商业可交付目标</span>
        </div>
        <Card.Root class="colophon">
          <p>Cullify 0.1.0 是面向摄影师的本地 AI 选片工具。当前版本已接入 Rust 扫描器、SQLite 持久化、快速质量评分、手动决策保存和 JSON / CSV / ZIP 导出；专家 Provider 推理会在后续版本继续增强。</p>
          <div>
            <span><b>桌面框架</b><em>Tauri 2</em></span>
            <span><b>前端</b><em>Svelte 5</em></span>
            <span><b>后端语言</b><em>Rust · Edition 2024</em></span>
            <span><b>数据目录</b><em>{appDataDir}</em></span>
          </div>
        </Card.Root>
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
        <Button variant="outline" onclick={() => void loadSavedConfig()} disabled={isSavingConfig}>
          <RotateCcwIcon data-icon="inline-start" aria-hidden="true" />
          放弃改动
        </Button>
        <Button onclick={() => void saveChanges()} disabled={isSavingConfig}>
          <SaveIcon data-icon="inline-start" aria-hidden="true" />
          {isSavingConfig ? '保存中' : '保存到 config.toml'}
        </Button>
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

  .settings-head span {
    color: var(--muted-foreground);
    font-size: 12px;
  }

  :global(.colophon) {
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    padding: 24px 28px;
  }

  :global(.colophon p) {
    margin: 0 0 16px;
    border-bottom: 1px solid var(--border-soft);
    color: var(--muted-foreground);
    line-height: 1.7;
    padding-bottom: 14px;
  }

  :global(.colophon div) {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0 24px;
  }

  :global(.colophon span) {
    display: flex;
    justify-content: space-between;
    border-bottom: 1px solid var(--border-soft);
    padding: 7px 0;
  }

  :global(.colophon b),
  :global(.colophon em) {
    font-family: var(--font-mono);
    font-size: 11px;
    font-style: normal;
    font-weight: 500;
  }

  :global(.colophon b) {
    color: var(--meta);
    text-transform: uppercase;
  }

  :global(.colophon em) {
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
    .settings-head,
    .save-bar {
      align-items: flex-start;
      flex-direction: column;
    }
  }

  @media (max-width: 760px) {
    .settings-wrap {
      padding: 24px 18px 96px;
    }

    :global(.colophon div) {
      grid-template-columns: 1fr;
    }
  }
</style>
