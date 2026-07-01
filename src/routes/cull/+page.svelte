<script lang="ts">
  import { resolve } from '$app/paths';
  import { onDestroy, onMount } from 'svelte';
  import InspectorPanel from '$lib/components/InspectorPanel.svelte';
  import PhotoTile from '$lib/components/PhotoTile.svelte';
  import { describeExportError } from '$lib/exportMessages';
  import {
    backendPhotoToPhoto,
    backendProjectToProject,
    defaultAppConfig,
    exportProjectWithPickedFolder,
    tryLoadAppConfig,
    tryRevealInFileManager,
    setPhotoDecision as persistPhotoDecision,
    setPhotoDecisions as persistPhotoDecisions,
    tryListPhotos,
    tryListProjects
  } from '$lib/backend';
  import { photos as seedPhotos, projects as mockProjects } from '$lib/mockData';
  import { getShellContext } from '$lib/shell.svelte';
  import type { Decision, FilterMode, Photo, Project } from '$lib/types';

  type SortMode = 'quality-desc' | 'quality-asc' | 'name-asc' | 'time-asc';
  type UndoEntry = { id: string; decision: Decision };

  const gridBatchSize = 120;
  const demoProject = mockProjects[0];
  const emptyProject: Project = {
    id: 'empty',
    name: '暂无项目',
    shortName: '暂无项目',
    path: '请选择照片文件夹',
    mode: 'quick',
    total: 0,
    kept: 0,
    culled: 0,
    status: 'paused',
    statusLabel: '待导入',
    backend: true
  };

  let project = $state<Project>({ ...demoProject });
  const initialSelectedId = seedPhotos[1]?.id ?? seedPhotos[0]?.id ?? '';

  let photos = $state<Photo[]>(seedPhotos.map((photo) => ({ ...photo })));
  let selectedId = $state(initialSelectedId);
  let filter = $state<FilterMode>('all');
  let activeGroup = $state('all');
  let sortMode = $state<SortMode>('quality-desc');
  let shortcutBindings = $state(defaultAppConfig.shortcuts.map((shortcut) => ({ ...shortcut, keys: [...shortcut.keys] })));
  let undoStack = $state<Array<{ entries: UndoEntry[] }>>([]);
  let isExporting = $state(false);
  let isLoadingProject = $state(false);
  let isBatchUpdating = $state(false);
  let isLightboxOpen = $state(false);
  let isLightboxZoomed = $state(false);
  let visiblePhotoLimit = $state(gridBatchSize);
  let exportMessage = $state('');
  let projectMessage = $state('');
  let lastExportZipPath = $state('');

  const selectedIndex = $derived(Math.max(0, photos.findIndex((photo) => photo.id === selectedId)));
  const selectedPhoto = $derived(photos[selectedIndex] ?? photos[0]);
  const currentPosition = $derived(photos.length ? selectedIndex + 1 : 0);
  const keptCount = $derived(photos.filter((photo) => photo.decision === 'keep').length);
  const culledCount = $derived(photos.filter((photo) => photo.decision === 'cull' || photo.decision === 'auto').length);
  const pendingCount = $derived(photos.filter((photo) => photo.decision === null).length);
  const groupedPhotos = $derived.by(() => {
    if (activeGroup === 'all') return photos;
    return photos.filter((photo) => groupIdForPhoto(photo) === activeGroup);
  });
  const filteredPhotos = $derived.by(() => {
    const filtered =
      filter === 'keep'
        ? groupedPhotos.filter((photo) => photo.decision === 'keep')
        : filter === 'cull'
          ? groupedPhotos.filter((photo) => photo.decision === 'cull' || photo.decision === 'auto')
          : filter === 'pending'
            ? groupedPhotos.filter((photo) => photo.decision === null)
            : groupedPhotos;

    return [...filtered].sort(comparePhotos);
  });
  const visibleFilteredPhotos = $derived(filteredPhotos.slice(0, visiblePhotoLimit));
  const hiddenPhotoCount = $derived(Math.max(0, filteredPhotos.length - visibleFilteredPhotos.length));
  const progressPercent = $derived(Math.round((keptCount / Math.max(1, photos.length)) * 100));
  const emptyTitle = $derived(photos.length ? '当前筛选没有照片' : '暂无可挑选照片');
  const emptyCopy = $derived(
    photos.length
      ? '切换筛选条件，或继续处理其它待定照片。'
      : '请从主控台导入包含 JPG、PNG、WebP 或 TIFF 的照片文件夹。'
  );
  const sidebarGroups = $derived(
    [
      { id: 'all', name: '全部照片', count: photos.length },
      ...Array.from(
        photos.reduce((groups, photo) => {
          const id = groupIdForPhoto(photo);
          const existing = groups.get(id);
          groups.set(id, {
            id,
            name: photo.group || '未分组',
            count: (existing?.count ?? 0) + 1
          });
          return groups;
        }, new Map<string, { id: string; name: string; count: number }>())
      ).map(([, group]) => group)
    ]
  );
  const sortOptions: Array<{ id: SortMode; label: string }> = [
    { id: 'quality-desc', label: '质量分 高到低' },
    { id: 'quality-asc', label: '质量分 低到高' },
    { id: 'name-asc', label: '文件名 A-Z' },
    { id: 'time-asc', label: '拍摄时间 早到晚' }
  ];
  const activeSortLabel = $derived(sortOptions.find((option) => option.id === sortMode)?.label ?? '质量分 高到低');
  const shell = getShellContext();

  const filters: Array<{ id: FilterMode; label: string; count: () => number }> = [
    { id: 'all', label: '全部', count: () => groupedPhotos.length },
    { id: 'keep', label: '保留', count: () => groupedPhotos.filter((photo) => photo.decision === 'keep').length },
    { id: 'cull', label: '淘汰', count: () => groupedPhotos.filter((photo) => photo.decision === 'cull' || photo.decision === 'auto').length },
    { id: 'pending', label: '待定', count: () => groupedPhotos.filter((photo) => photo.decision === null).length }
  ];

  onMount(() => {
    void loadShortcuts();
    loadActiveProject();
  });

  onDestroy(() => {
    shell.resetPage();
  });

  $effect(() => {
    shell.configure({
      active: 'cull',
      title: '照片挑选',
      subtitle: `${project.shortName} · ${project.mode} · ${photos.length.toLocaleString()} 张`,
      cullCount: String(photos.length),
      sidebarExtra: cullSidebar,
      showDefaultSidebarDetails: false,
      footer: {
        photo: `照片 ${currentPosition} / ${photos.length}`,
        analysis: `${project.statusLabel} · 保留 ${keptCount} · 淘汰 ${culledCount} · 待定 ${pendingCount}`,
        model: project.mode === 'quick' ? '快速模式 · Rust 原生分析' : `${project.mode} · 模型队列待接入`,
        resources: {
          cpu: isLoadingProject || isBatchUpdating ? 'CPU 写入中' : 'CPU 待机',
          memory: `内存 已加载 ${photos.length.toLocaleString()} 张`,
          gpu: '显存 未占用'
        }
      }
    });
  });

  async function loadShortcuts() {
    const envelope = await tryLoadAppConfig();
    shortcutBindings = (envelope?.config.shortcuts ?? defaultAppConfig.shortcuts).map((shortcut) => ({
      ...shortcut,
      keys: [...shortcut.keys]
    }));
  }

  async function loadActiveProject() {
    isLoadingProject = true;
    projectMessage = '';

    try {
      const backendProjects = await tryListProjects();
      if (backendProjects === null) return;
      if (backendProjects.length === 0) {
        project = { ...emptyProject };
        photos = [];
        selectedId = '';
        visiblePhotoLimit = gridBatchSize;
        projectMessage = '还没有本地项目，请先从主控台导入包含图片的文件夹。';
        return;
      }

      const activeProjectId = localStorage.getItem('cullify.activeProjectId');
      const backendProject =
        backendProjects.find((item) => item.id === activeProjectId) ?? backendProjects[0];
      project = backendProjectToProject(backendProject);
      localStorage.setItem('cullify.activeProjectId', backendProject.id);

      const backendPhotos = await tryListPhotos(backendProject.id);
      if (backendPhotos) {
        photos = backendPhotos.map(backendPhotoToPhoto);
        selectedId = photos[0]?.id ?? '';
        visiblePhotoLimit = gridBatchSize;
        projectMessage = photos.length
          ? ''
          : '这个项目还没有可挑选照片，请从主控台重新导入包含图片的文件夹。';
      } else {
        photos = [];
        selectedId = '';
        visiblePhotoLimit = gridBatchSize;
        projectMessage = '无法加载这个项目的照片，请确认原始文件夹仍可访问。';
      }
    } finally {
      isLoadingProject = false;
    }
  }

  function choosePhoto(photo: Photo) {
    selectedId = photo.id;
  }

  function openLightbox() {
    if (!selectedPhoto) return;
    isLightboxOpen = true;
    isLightboxZoomed = false;
  }

  function closeLightbox() {
    isLightboxOpen = false;
    isLightboxZoomed = false;
  }

  function toggleLightbox() {
    if (isLightboxOpen) {
      closeLightbox();
    } else {
      openLightbox();
    }
  }

  function chooseGroup(groupId: string) {
    activeGroup = groupId;
    resetVisiblePhotos();
    selectFirstVisiblePhoto();
  }

  function chooseFilter(filterId: FilterMode) {
    filter = filterId;
    resetVisiblePhotos();
    selectFirstVisiblePhoto();
  }

  function cycleSortMode() {
    const currentIndex = sortOptions.findIndex((option) => option.id === sortMode);
    sortMode = sortOptions[(currentIndex + 1) % sortOptions.length].id;
    resetVisiblePhotos();
    selectFirstVisiblePhoto();
  }

  async function setDecision(decision: Decision) {
    const photo = photos.find((item) => item.id === selectedId);
    if (!photo) return;
    const previousDecision = photo.decision;
    undoStack.push({ entries: [{ id: photo.id, decision: previousDecision }] });
    photo.decision = decision;
    if (photo.backendId && (decision === 'keep' || decision === 'cull' || decision === null)) {
      persistPhotoDecision(photo.backendId, decision).catch((error) => {
        console.info('Unable to persist photo decision.', error);
        photo.decision = previousDecision;
        projectMessage = '保存这张照片的决策失败，已恢复本地状态。';
      });
    }
    if (decision) moveBy(1);
  }

  async function setVisibleDecisions(decision: Decision) {
    if (isBatchUpdating || filteredPhotos.length === 0) return;
    const targets = filteredPhotos.filter((photo) => photo.decision !== decision);
    if (targets.length === 0) {
      projectMessage = '当前筛选结果已经是这个标记状态。';
      return;
    }

    const label = decision === 'keep' ? '保留' : decision === 'cull' ? '淘汰' : '待定';
    if (decision !== null) {
      const confirmed = window.confirm(`将当前筛选出的 ${targets.length} 张照片全部标记为「${label}」？`);
      if (!confirmed) return;
    }

    isBatchUpdating = true;
    projectMessage = '';
    const previousEntries = targets.map((photo) => ({ id: photo.id, decision: photo.decision }));
    undoStack.push({ entries: previousEntries });
    for (const photo of targets) {
      photo.decision = decision;
    }

    try {
      const backendIds = targets
        .map((photo) => photo.backendId)
        .filter((id): id is string => Boolean(id));
      if (project.backend && backendIds.length > 0) {
        await persistPhotoDecisions({
          projectId: project.id,
          photoIds: backendIds,
          decision: decision === 'auto' ? null : decision
        });
      }
      projectMessage = `已将当前筛选的 ${targets.length} 张照片标记为「${label}」。`;
      selectFirstVisiblePhoto();
    } catch (error) {
      for (const entry of previousEntries) {
        const photo = photos.find((item) => item.id === entry.id);
        if (photo) photo.decision = entry.decision;
      }
      undoStack.pop();
      projectMessage = describeDecisionError(error);
    } finally {
      isBatchUpdating = false;
    }
  }

  async function undo() {
    const last = undoStack.pop();
    if (!last) return;
    const photosToPersist: Array<{ photo: Photo; decision: Decision }> = [];
    for (const entry of last.entries) {
      const photo = photos.find((item) => item.id === entry.id);
      if (!photo) continue;
      photo.decision = entry.decision;
      photosToPersist.push({ photo, decision: entry.decision });
    }

    selectedId = last.entries[0]?.id ?? selectedId;
    const backendIds = photosToPersist
      .map(({ photo }) => photo.backendId)
      .filter((id): id is string => Boolean(id));
    const decisions = new Set(photosToPersist.map(({ decision }) => decision));
    if (project.backend && backendIds.length > 0 && decisions.size === 1) {
      const [decision] = decisions;
      persistPhotoDecisions({
        projectId: project.id,
        photoIds: backendIds,
        decision: decision === 'auto' ? null : decision
      }).catch((error) => {
        console.info('Unable to persist batch undo decision.', error);
        projectMessage = '撤销已在界面生效，但保存到本地数据库失败。';
      });
    } else {
      for (const { photo, decision } of photosToPersist) {
        if (photo.backendId && (decision === 'keep' || decision === 'cull' || decision === null)) {
          persistPhotoDecision(photo.backendId, decision).catch((error) => {
            console.info('Unable to persist undo decision.', error);
            projectMessage = '撤销已在界面生效，但保存到本地数据库失败。';
          });
        }
      }
    }
  }

  async function exportCurrentProject() {
    if (isExporting) return;
    if (photos.length === 0) {
      exportMessage = '当前项目没有可导出的照片。';
      lastExportZipPath = '';
      return;
    }

    if (!project.backend) {
      exportMessage = '导出需要在 Tauri 桌面环境中打开真实项目。';
      lastExportZipPath = '';
      return;
    }

    isExporting = true;
    exportMessage = '';
    lastExportZipPath = '';
    const outcome = await exportProjectWithPickedFolder(project.id);
    if (outcome.status === 'exported') {
      const { summary } = outcome;
      exportMessage = `已导出 ${summary.total} 张 · ZIP / CSV / JSON 位于 ${summary.exportDir}`;
      lastExportZipPath = summary.zipPath;
    } else if (outcome.status === 'cancelled') {
      exportMessage = '已取消导出。';
    } else {
      exportMessage = describeExportError(outcome.error);
    }
    isExporting = false;
  }

  async function revealLastExport() {
    if (!lastExportZipPath) return;
    const opened = await tryRevealInFileManager(lastExportZipPath);
    if (!opened) exportMessage = '无法打开导出目录，请手动前往导出路径查看。';
  }

  function moveBy(delta: number) {
    if (filteredPhotos.length === 0) return;
    const visibleIndex = Math.max(0, filteredPhotos.findIndex((photo) => photo.id === selectedId));
    const next = Math.min(Math.max(visibleIndex + delta, 0), filteredPhotos.length - 1);
    selectedId = filteredPhotos[next].id;
    ensurePhotoIndexRendered(next);
    isLightboxZoomed = false;
  }

  function loadMoreVisiblePhotos() {
    visiblePhotoLimit = Math.min(filteredPhotos.length, visiblePhotoLimit + gridBatchSize);
  }

  function resetVisiblePhotos() {
    visiblePhotoLimit = gridBatchSize;
  }

  function ensurePhotoIndexRendered(index: number) {
    if (index < visiblePhotoLimit - 12) return;
    visiblePhotoLimit = Math.min(filteredPhotos.length, Math.max(visiblePhotoLimit, index + gridBatchSize));
  }

  function handleGridScroll(event: Event) {
    const element = event.currentTarget;
    if (!(element instanceof HTMLElement)) return;
    const distanceToBottom = element.scrollHeight - element.scrollTop - element.clientHeight;
    if (distanceToBottom < 360 && hiddenPhotoCount > 0) loadMoreVisiblePhotos();
  }

  function handleKeydown(event: KeyboardEvent) {
    const target = event.target;
    if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement || target instanceof HTMLSelectElement) {
      return;
    }

    if (event.key === 'Escape' && isLightboxOpen) {
      event.preventDefault();
      closeLightbox();
      return;
    }

    if (matchesShortcut('undo', event)) {
      event.preventDefault();
      void undo();
      return;
    }

    const normalizedKey = normalizeKey(event.key);
    const navKeys = keysForShortcut('nav');
    if (normalizedKey === navKeys[0]) {
      event.preventDefault();
      moveBy(-1);
      return;
    }
    if (normalizedKey === navKeys[1]) {
      event.preventDefault();
      moveBy(1);
      return;
    }

    if (matchesShortcut('fullscreen', event)) {
      event.preventDefault();
      toggleLightbox();
      return;
    }

    if (matchesShortcut('keep', event)) {
      event.preventDefault();
      void setDecision('keep');
      return;
    }

    if (matchesShortcut('cull', event)) {
      event.preventDefault();
      void setDecision('cull');
      return;
    }

    if (matchesShortcut('skip', event)) {
      event.preventDefault();
      void setDecision(null);
      moveBy(1);
      return;
    }

    const allKeys = keysForShortcut('all');
    if (normalizedKey === allKeys[0]) {
      event.preventDefault();
      void setVisibleDecisions('keep');
      return;
    }
    if (normalizedKey === allKeys[1]) {
      event.preventDefault();
      void setVisibleDecisions('cull');
      return;
    }

    if (matchesShortcut('grid', event)) {
      event.preventDefault();
      chooseFilter(filter === 'all' ? 'pending' : 'all');
    }
  }

  function selectFirstVisiblePhoto() {
    const next = filteredPhotos[0];
    if (next) selectedId = next.id;
  }

  function comparePhotos(a: Photo, b: Photo) {
    if (sortMode === 'quality-asc') return a.score - b.score || a.name.localeCompare(b.name);
    if (sortMode === 'name-asc') return a.name.localeCompare(b.name);
    if (sortMode === 'time-asc') return a.time.localeCompare(b.time) || a.name.localeCompare(b.name);
    return b.score - a.score || a.name.localeCompare(b.name);
  }

  function groupIdForPhoto(photo: Photo) {
    return slugify(photo.group || '未分组');
  }

  function slugify(value: string) {
    const slug = value
      .toLowerCase()
      .replace(/[^a-z0-9\u4e00-\u9fff]+/g, '-')
      .replace(/^-+|-+$/g, '');
    return slug || 'ungrouped';
  }

  function keysForShortcut(id: string) {
    return shortcutBindings.find((shortcut) => shortcut.id === id)?.keys ?? [];
  }

  function shortcutLabel(id: string) {
    return keysForShortcut(id).map(displayShortcutKey).join(' + ');
  }

  function displayShortcutKey(key: string) {
    if (key === 'Left') return '←';
    if (key === 'Right') return '→';
    if (key === 'Up') return '↑';
    if (key === 'Down') return '↓';
    if (key === 'Cmd') return '⌘';
    if (key === 'Space') return 'Space';
    return key;
  }

  function matchesShortcut(id: string, event: KeyboardEvent) {
    const keys = keysForShortcut(id);
    if (keys.length === 0) return false;

    const keySet = new Set(keys.map((key) => key.toLowerCase()));
    const wantsCommand = keySet.has('cmd') || keySet.has('ctrl') || keySet.has('control');
    const wantsShift = keySet.has('shift');
    const wantsAlt = keySet.has('alt') || keySet.has('option');
    const nonModifierKeys = keys.filter((key) => !['cmd', 'ctrl', 'control', 'shift', 'alt', 'option'].includes(key.toLowerCase()));
    const keyMatches = nonModifierKeys.length === 0 || nonModifierKeys.includes(normalizeKey(event.key));

    if (!keyMatches) return false;
    if (wantsCommand !== (event.metaKey || event.ctrlKey)) return false;
    if (wantsShift !== event.shiftKey) return false;
    if (wantsAlt !== event.altKey) return false;
    return wantsCommand || (!event.metaKey && !event.ctrlKey);
  }

  function normalizeKey(key: string) {
    if (key === ' ') return 'Space';
    if (key === 'ArrowLeft') return 'Left';
    if (key === 'ArrowRight') return 'Right';
    if (key === 'ArrowUp') return 'Up';
    if (key === 'ArrowDown') return 'Down';
    if (key === 'Meta') return 'Cmd';
    if (key === 'Control') return 'Ctrl';
    return key.length === 1 ? key.toUpperCase() : key;
  }

  function describeDecisionError(error: unknown) {
    const message = error instanceof Error ? error.message : typeof error === 'string' ? error : 'unknown error';
    if (message.includes('invalid decision')) return '标记失败：决策值无效。';
    if (message.includes('photo batch update mismatch')) return '批量标记失败：部分照片不属于当前项目，已恢复本地状态。';
    if (message.includes('__TAURI__') || message.includes('invoke') || message.includes('not available')) {
      return '批量标记需要在 Tauri 桌面环境中运行真实项目。';
    }
    return `批量标记失败：${message}`;
  }
</script>

<svelte:window onkeydown={handleKeydown} />

{#snippet cullSidebar()}
  <div class="mode-switch">
    <span class={project.mode === 'quick' ? 'active' : ''}>快速</span>
    <span class={project.mode === 'expert' ? 'active' : 'planned'}>专家</span>
    <span class={project.mode === 'arena' ? 'active' : 'planned'}>竞技场</span>
  </div>

  <div class="project-meta">
    <p class="meta-heading">项目</p>
    <div class="meta-row"><span>名称</span><span class="meta-value">{project.shortName}</span></div>
    <div class="meta-row"><span>模式</span><span class="meta-value">{project.mode}</span></div>
    <div class="meta-row"><span>状态</span><span class="meta-value">{project.statusLabel}</span></div>
    <p class="meta-heading">照片</p>
    <div class="meta-row"><span>总数</span><span class="meta-value">{photos.length}</span></div>
    <div class="meta-row"><span>保留</span><span class="meta-value">{keptCount}</span></div>
    <div class="meta-row"><span>淘汰</span><span class="meta-value">{culledCount}</span></div>
    <div class="meta-row"><span>待定</span><span class="meta-value">{pendingCount}</span></div>
  </div>

  <p class="nav-eyebrow group-title">连拍组</p>
  <div class="group-list">
    {#each sidebarGroups as group (group.id)}
      <button
        type="button"
        class={['group-item', activeGroup === group.id && 'active'].filter(Boolean).join(' ')}
        onclick={() => chooseGroup(group.id)}
      >
        <span>{group.name}</span>
        <span>{group.count}</span>
      </button>
    {/each}
  </div>
{/snippet}

<main class="app-main cull-main">
    <section class="workspace">
      <header class="toolbar">
        <div class="toolbar-left">
          <h1 class="panel-title">{project.name}</h1>
          <span class={`tag tag-${project.mode}`}>{project.mode}</span>
        </div>
        <div class="toolbar-right">
          <div class="filter-group">
            {#each filters as item (item.id)}
              <button
                type="button"
                class={filter === item.id ? 'active' : ''}
                onclick={() => chooseFilter(item.id)}
              >
                {item.label}<span>{item.count()}</span>
              </button>
            {/each}
          </div>
          <button class="btn btn-secondary sort-button" type="button" onclick={cycleSortMode}>
            排序 · {activeSortLabel}
          </button>
          <button class="btn btn-secondary" type="button" onclick={openLightbox} disabled={!selectedPhoto}>
            灯箱 · {shortcutLabel('fullscreen')}
          </button>
          <div class="batch-actions" aria-label="批量标记当前筛选照片">
            <button type="button" onclick={() => void setVisibleDecisions('keep')} disabled={isBatchUpdating || filteredPhotos.length === 0}>
              当前全要
            </button>
            <button type="button" class="danger" onclick={() => void setVisibleDecisions('cull')} disabled={isBatchUpdating || filteredPhotos.length === 0}>
              当前全不要
            </button>
            <button type="button" onclick={() => void setVisibleDecisions(null)} disabled={isBatchUpdating || filteredPhotos.length === 0}>
              清空标记
            </button>
          </div>
          <button class="btn btn-ghost export-button" type="button" onclick={() => void exportCurrentProject()} disabled={isExporting || isBatchUpdating}>
            {isExporting ? '导出中' : '导出'}
          </button>
        </div>
      </header>

      {#if exportMessage}
        <div class="export-note">
          <span>{exportMessage}</span>
          {#if lastExportZipPath}
            <button type="button" onclick={() => void revealLastExport()}>打开导出目录</button>
          {/if}
        </div>
      {/if}

      {#if isLoadingProject || projectMessage}
        <div class="project-note" role="status">
          {isLoadingProject ? '正在加载项目照片...' : projectMessage}
        </div>
      {/if}

      <div class="progress">
        <span class="num">进度</span>
        <div class="progress-bar" aria-hidden="true"><span style:width={`${progressPercent}%`}></span></div>
        <div class="progress-stats">
          <span class="keep">保留 {keptCount}</span>
          <span class="cull">淘汰 {culledCount}</span>
          <span>待定 {pendingCount}</span>
        </div>
        <span class="num">{progressPercent}%</span>
      </div>

      <div class="grid-wrap" onscroll={handleGridScroll}>
        {#if filteredPhotos.length > 0}
          <div class="photo-grid">
            {#each visibleFilteredPhotos as photo (photo.id)}
              <PhotoTile photo={photo} selected={photo.id === selectedId} onclick={choosePhoto} />
            {/each}
          </div>
          {#if hiddenPhotoCount > 0}
            <div class="load-more">
              <span>已渲染 {visibleFilteredPhotos.length.toLocaleString()} / {filteredPhotos.length.toLocaleString()} 张</span>
              <button type="button" onclick={loadMoreVisiblePhotos}>加载更多</button>
            </div>
          {/if}
        {:else}
          <div class="empty-state">
            <strong>{emptyTitle}</strong>
            <span>{emptyCopy}</span>
            <a class="btn btn-secondary" href={resolve('/')}>返回主控台</a>
          </div>
        {/if}
      </div>

      {#if selectedPhoto}
        <div class="inspector-dock">
          <InspectorPanel
            photo={selectedPhoto}
            index={selectedIndex}
            total={photos.length}
            shortcuts={shortcutBindings}
            onDecision={(decision) => void setDecision(decision)}
          />
        </div>
      {/if}
    </section>
  </main>

{#if isLightboxOpen && selectedPhoto}
    <div class="lightbox" role="dialog" aria-modal="true" aria-label={`灯箱查看 ${selectedPhoto.name}`}>
      <div class="lightbox-top">
        <div>
          <strong>{selectedPhoto.name}</strong>
          <span>{currentPosition} / {photos.length} · {selectedPhoto.score}/100 · {selectedPhoto.size}</span>
        </div>
        <div class="lightbox-actions">
          <button type="button" onclick={() => moveBy(-1)} disabled={filteredPhotos.length <= 1}>上一张</button>
          <button type="button" onclick={() => moveBy(1)} disabled={filteredPhotos.length <= 1}>下一张</button>
          <button type="button" onclick={() => (isLightboxZoomed = !isLightboxZoomed)}>
            {isLightboxZoomed ? '适合窗口' : '100%'}
          </button>
          <button type="button" onclick={closeLightbox}>关闭</button>
        </div>
      </div>

      <button class="lightbox-stage" type="button" onclick={() => (isLightboxZoomed = !isLightboxZoomed)}>
        {#if selectedPhoto.originalUrl || selectedPhoto.sourceUrl}
          <img
            class={['lightbox-image', isLightboxZoomed && 'zoomed'].filter(Boolean).join(' ')}
            src={selectedPhoto.originalUrl ?? selectedPhoto.sourceUrl}
            alt={selectedPhoto.name}
          />
        {:else}
          <span class="lightbox-placeholder" style:background={selectedPhoto.palette}></span>
        {/if}
      </button>

      <div class="lightbox-bottom">
        <div class="lightbox-meta">
          <span>{selectedPhoto.camera}</span>
          <span>{selectedPhoto.lens}</span>
          <span>{selectedPhoto.focal}</span>
          <span>{selectedPhoto.aperture}</span>
          <span>{selectedPhoto.shutter}</span>
          <span>ISO {selectedPhoto.iso}</span>
        </div>
        <div class="lightbox-decisions">
          <button type="button" class="keep" onclick={() => void setDecision('keep')}>保留 · {shortcutLabel('keep')}</button>
          <button type="button" class="cull" onclick={() => void setDecision('cull')}>淘汰 · {shortcutLabel('cull')}</button>
          <button type="button" onclick={() => void setDecision(null)}>待定 · {shortcutLabel('skip')}</button>
        </div>
      </div>
    </div>
{/if}

<style>
  .cull-main {
    display: block;
    overflow: hidden;
  }

  .cull-shell {
    grid-template-columns: 248px minmax(0, 1fr);
  }

  .mode-switch {
    display: flex;
    gap: 2px;
    margin-bottom: 18px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
    padding: 3px;
  }

  .mode-switch span {
    flex: 1;
    border-radius: 5px;
    background: transparent;
    color: var(--muted);
    font-size: 12px;
    font-weight: 700;
    padding: 7px 8px;
    text-align: center;
  }

  .mode-switch .active {
    background: var(--surface);
    box-shadow: 0 0 0 1px var(--border);
    color: var(--accent);
  }

  .mode-switch .planned {
    color: color-mix(in srgb, var(--meta) 72%, transparent);
  }

  .meta-heading {
    margin: 8px 0 6px;
    border-top: 1px solid var(--border-soft);
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 1.1px;
    padding-top: 8px;
    text-transform: uppercase;
  }

  .meta-heading:first-child {
    margin-top: 0;
    border-top: 0;
    padding-top: 0;
  }

  .meta-row {
    color: var(--muted);
    font-size: 12px;
    padding: 3px 0;
  }

  .meta-value {
    color: var(--fg-2);
    font-size: 11px;
  }

  .group-title {
    margin-top: 20px;
  }

  .group-list {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .group-item {
    display: flex;
    justify-content: space-between;
    width: 100%;
    border-radius: 5px;
    background: transparent;
    color: var(--fg-2);
    font-size: 12px;
    padding: 7px 10px;
    text-align: left;
  }

  .group-item:hover,
  .group-item.active {
    background: var(--tag-bg-soft);
    color: var(--accent);
  }

  .group-item span:last-child {
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 10px;
  }

  .workspace {
    display: flex;
    height: 100%;
    min-width: 0;
    min-height: 0;
    flex-direction: column;
    overflow: hidden;
  }

  .toolbar {
    flex-wrap: wrap;
    flex-shrink: 0;
    align-items: flex-start;
    gap: 16px;
    border-bottom: 1px solid var(--border);
    padding: 14px 24px;
  }

  .toolbar-left,
  .toolbar-right {
    gap: 14px;
  }

  .toolbar-left {
    flex: 0 0 auto;
  }

  .toolbar-right {
    min-width: min(100%, 640px);
    flex: 1 1 640px;
    flex-wrap: wrap;
    justify-content: flex-end;
  }

  .toolbar-right button {
    flex: 0 0 auto;
    white-space: nowrap;
  }

  .export-button {
    min-width: 56px;
    white-space: nowrap;
  }

  .panel-title {
    margin: 0;
    font-size: 17px;
  }

  .filter-group {
    display: flex;
    gap: 2px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    padding: 2px;
  }

  .filter-group button {
    border-radius: 4px;
    background: transparent;
    color: var(--muted);
    font-size: 12px;
    padding: 5px 10px;
  }

  .filter-group button.active {
    background: var(--surface-warm);
    color: var(--fg);
  }

  .filter-group span {
    margin-left: 4px;
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 10px;
  }

  .batch-actions {
    display: flex;
    gap: 2px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    padding: 2px;
  }

  .batch-actions button {
    border-radius: 4px;
    background: transparent;
    color: var(--accent);
    font-size: 12px;
    font-weight: 700;
    padding: 5px 8px;
    white-space: nowrap;
  }

  .batch-actions button:hover {
    background: var(--surface-warm);
  }

  .batch-actions button.danger {
    color: var(--danger);
  }

  .batch-actions button:disabled {
    cursor: wait;
    opacity: 0.52;
  }

  .progress {
    display: flex;
    flex-shrink: 0;
    align-items: center;
    gap: 14px;
    border-bottom: 1px solid var(--border-soft);
    background: var(--surface);
    color: var(--meta);
    font-size: 11px;
    padding: 8px 24px;
  }

  .export-note {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    margin: -8px 24px 16px;
    border: 1px solid var(--border);
    border-left: 2px solid var(--accent);
    border-radius: 0 6px 6px 0;
    background: var(--surface);
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
    padding: 10px 12px;
  }

  .project-note {
    margin: -8px 24px 16px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--muted);
    font-size: 12px;
    line-height: 1.5;
    padding: 10px 12px;
  }

  .export-note span {
    min-width: 0;
  }

  .export-note button {
    flex: 0 0 auto;
    border: 1px solid var(--border);
    border-radius: 5px;
    background: var(--bg);
    color: var(--accent);
    font-size: 12px;
    font-weight: 700;
    padding: 6px 10px;
  }

  .progress-bar {
    position: relative;
    flex: 1;
    height: 4px;
    overflow: hidden;
    border-radius: 2px;
    background: var(--border);
  }

  .progress-bar span {
    position: absolute;
    inset: 0 auto 0 0;
    background: var(--accent);
  }

  .progress-stats {
    display: flex;
    gap: 18px;
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
  }

  .progress-stats .keep {
    color: var(--accent);
  }

  .progress-stats .cull {
    color: var(--danger);
  }

  .grid-wrap {
    flex: 1;
    min-height: 0;
    overflow-y: auto;
    padding: 16px 24px 24px;
  }

  .photo-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(188px, 1fr));
    gap: 10px;
  }

  .inspector-dock {
    flex: 0 0 auto;
    border-top: 1px solid var(--border);
    background: var(--surface);
  }

  .load-more {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    margin-top: 16px;
    border: 1px dashed var(--border);
    border-radius: 8px;
    background: var(--surface);
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 11px;
    padding: 12px;
  }

  .load-more button {
    border: 1px solid var(--border);
    border-radius: 5px;
    background: var(--bg);
    color: var(--accent);
    font-size: 12px;
    font-weight: 700;
    padding: 6px 10px;
  }

  .load-more button:hover {
    background: var(--surface-warm);
  }

  .lightbox {
    position: fixed;
    z-index: 40;
    inset: 0;
    display: grid;
    grid-template-rows: auto minmax(0, 1fr) auto;
    background: rgba(12, 12, 11, 0.94);
    color: var(--surface);
  }

  .lightbox-top,
  .lightbox-bottom {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    border-color: rgba(255, 255, 255, 0.12);
    padding: 12px 18px;
  }

  .lightbox-top {
    border-bottom: 1px solid rgba(255, 255, 255, 0.12);
  }

  .lightbox-bottom {
    border-top: 1px solid rgba(255, 255, 255, 0.12);
  }

  .lightbox-top strong {
    display: block;
    font-family: var(--font-display);
    font-size: 17px;
    font-weight: 500;
  }

  .lightbox-top span,
  .lightbox-meta {
    color: rgba(250, 249, 245, 0.72);
    font-family: var(--font-mono);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }

  .lightbox-actions,
  .lightbox-decisions,
  .lightbox-meta {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .lightbox-actions button,
  .lightbox-decisions button {
    border: 1px solid rgba(255, 255, 255, 0.16);
    border-radius: 5px;
    background: rgba(250, 249, 245, 0.08);
    color: var(--surface);
    font-size: 12px;
    font-weight: 700;
    padding: 7px 10px;
  }

  .lightbox-actions button:hover,
  .lightbox-decisions button:hover {
    background: rgba(250, 249, 245, 0.14);
  }

  .lightbox-actions button:disabled {
    cursor: default;
    opacity: 0.42;
  }

  .lightbox-decisions .keep {
    border-color: color-mix(in srgb, var(--success) 70%, transparent);
    background: color-mix(in srgb, var(--success) 78%, transparent);
  }

  .lightbox-decisions .cull {
    border-color: color-mix(in srgb, var(--danger) 70%, transparent);
    background: color-mix(in srgb, var(--danger) 78%, transparent);
  }

  .lightbox-stage {
    display: grid;
    min-width: 0;
    min-height: 0;
    place-items: center;
    overflow: auto;
    background:
      linear-gradient(45deg, rgba(255, 255, 255, 0.045) 25%, transparent 25%),
      linear-gradient(-45deg, rgba(255, 255, 255, 0.045) 25%, transparent 25%),
      linear-gradient(45deg, transparent 75%, rgba(255, 255, 255, 0.045) 75%),
      linear-gradient(-45deg, transparent 75%, rgba(255, 255, 255, 0.045) 75%);
    background-position: 0 0, 0 8px, 8px -8px, -8px 0;
    background-size: 16px 16px;
    cursor: zoom-in;
    padding: 18px;
  }

  .lightbox-image {
    display: block;
    max-width: 100%;
    max-height: 100%;
    object-fit: contain;
    box-shadow: 0 24px 80px rgba(0, 0, 0, 0.32);
  }

  .lightbox-image.zoomed {
    max-width: none;
    max-height: none;
    cursor: zoom-out;
  }

  .lightbox-placeholder {
    width: min(78vw, 1200px);
    aspect-ratio: 4 / 3;
    border: 1px solid rgba(255, 255, 255, 0.14);
    border-radius: 6px;
  }

  .empty-state {
    display: grid;
    min-height: 320px;
    place-items: center;
    align-content: center;
    gap: 12px;
    border: 1px dashed var(--border);
    border-radius: 8px;
    background: var(--surface);
    color: var(--muted);
    text-align: center;
  }

  .empty-state strong {
    color: var(--fg);
    font-family: var(--font-display);
    font-size: 20px;
    font-weight: 500;
  }

  .empty-state span {
    max-width: 380px;
    font-size: 13px;
    line-height: 1.5;
  }

  @media (max-width: 760px) {
    .toolbar,
    .toolbar-right,
    .progress {
      align-items: stretch;
      flex-direction: column;
    }

    .filter-group {
      overflow-x: auto;
    }

    .batch-actions {
      overflow-x: auto;
    }

    .lightbox-top,
    .lightbox-bottom {
      align-items: flex-start;
      flex-direction: column;
    }

    .lightbox-actions,
    .lightbox-decisions,
    .lightbox-meta {
      flex-wrap: wrap;
    }

    .photo-grid {
      grid-template-columns: repeat(auto-fill, minmax(140px, 1fr));
    }

    .load-more {
      align-items: stretch;
      flex-direction: column;
      text-align: center;
    }
  }
</style>
