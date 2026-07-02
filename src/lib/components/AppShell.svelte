<script lang="ts">
  import { resolve } from '$app/paths';
  import { onMount } from 'svelte';
  import { tryLoadSystemResourceSnapshot, type SystemResourceSnapshot } from '$lib/backend';
  import type { Snippet } from 'svelte';
  import type { ShellController } from '$lib/shell.svelte';
  import type { AppRoute } from '$lib/types';

  interface Props {
    shell: ShellController;
    children: Snippet;
  }

  let { shell, children }: Props = $props();

  type RoutePath = '/' | '/cull' | '/settings';

  const navItems: Array<{ id: AppRoute; label: string; href: RoutePath; count?: () => string }> = [
    { id: 'dashboard', label: '主控台', href: '/', count: () => '·' },
    { id: 'cull', label: '照片挑选', href: '/cull', count: () => shell.cullCount },
    { id: 'settings', label: '设置', href: '/settings', count: () => '·' }
  ];

  const sidebarMin = 188;
  const sidebarMax = 360;
  const collapseBelow = 172;
  let sidebarWidth = $state(248);
  let sidebarCollapsed = $state(false);
  let platformClass = $state('platform-other');
  let runtimeClass = $state('browser-runtime');
  let resourceSnapshot = $state<SystemResourceSnapshot | null>(null);
  const titlebarInteractiveSelector = 'button, a, input, select, textarea, [role="button"]';
  const cpuLabel = $derived(
    resourceSnapshot ? `CPU ${Math.round(resourceSnapshot.cpuUsage)}%` : shell.footer.resources.cpu
  );
  const memoryLabel = $derived(
    resourceSnapshot
      ? `内存 ${formatBytes(resourceSnapshot.appMemoryBytes)} / ${formatBytes(resourceSnapshot.usedMemoryBytes)}`
      : shell.footer.resources.memory
  );

  onMount(() => {
    const saved = Number(localStorage.getItem('cullify.sidebarWidth'));
    const savedCollapsed = localStorage.getItem('cullify.sidebarCollapsed') === 'true';
    if (Number.isFinite(saved) && saved >= sidebarMin && saved <= sidebarMax) {
      sidebarWidth = saved;
    }
    sidebarCollapsed = savedCollapsed;
    platformClass = platformFromUserAgent(navigator.userAgent);
    runtimeClass = '__TAURI_INTERNALS__' in window ? 'tauri-runtime' : 'browser-runtime';
    void refreshResourceSnapshot();
    const resourceTimer = window.setInterval(() => void refreshResourceSnapshot(), 3000);
    return () => window.clearInterval(resourceTimer);
  });

  function platformFromUserAgent(userAgent: string) {
    if (/Macintosh|Mac OS X/.test(userAgent)) return 'platform-mac';
    if (/Windows/.test(userAgent)) return 'platform-windows';
    return 'platform-other';
  }

  function toggleSidebar() {
    sidebarCollapsed = !sidebarCollapsed;
    localStorage.setItem('cullify.sidebarCollapsed', String(sidebarCollapsed));
  }

  async function startTitlebarDrag(event: PointerEvent) {
    if (event.button !== 0 || isInteractiveTarget(event.target)) return;
    if (!('__TAURI_INTERNALS__' in window)) return;
    if (isMacTrafficLightArea(event)) return;
    event.preventDefault();
    try {
      const { getCurrentWindow } = await import('@tauri-apps/api/window');
      await getCurrentWindow().startDragging();
    } catch (error) {
      console.info('Unable to start Cullify window dragging.', error);
    }
  }

  function isMacTrafficLightArea(event: PointerEvent) {
    return platformClass === 'platform-mac' && runtimeClass === 'tauri-runtime' && event.clientX < 76 && event.clientY < 36;
  }

  function isInteractiveTarget(target: EventTarget | null) {
    return target instanceof Element && Boolean(target.closest(titlebarInteractiveSelector));
  }

  async function refreshResourceSnapshot() {
    const snapshot = await tryLoadSystemResourceSnapshot();
    if (snapshot) resourceSnapshot = snapshot;
  }

  function formatBytes(value: number) {
    if (value < 1024 * 1024) return `${Math.max(1, Math.round(value / 1024))} KB`;
    if (value < 1024 * 1024 * 1024) return `${Math.round(value / 1024 / 1024)} MB`;
    return `${(value / 1024 / 1024 / 1024).toFixed(1)} GB`;
  }

  function startSidebarResize(event: PointerEvent) {
    event.preventDefault();
    const startX = event.clientX;
    const startWidth = sidebarWidth;
    const pointerId = event.pointerId;
    const target = event.currentTarget;
    if (target instanceof HTMLElement) target.setPointerCapture(pointerId);

    function resize(moveEvent: PointerEvent) {
      const next = Math.max(0, Math.min(sidebarMax, startWidth + moveEvent.clientX - startX));
      if (next < collapseBelow) {
        sidebarCollapsed = true;
        return;
      }
      sidebarCollapsed = false;
      sidebarWidth = Math.max(sidebarMin, next);
    }

    function stopResize() {
      localStorage.setItem('cullify.sidebarWidth', String(sidebarWidth));
      localStorage.setItem('cullify.sidebarCollapsed', String(sidebarCollapsed));
      window.removeEventListener('pointermove', resize);
      window.removeEventListener('pointerup', stopResize);
      window.removeEventListener('pointercancel', stopResize);
    }

    window.addEventListener('pointermove', resize);
    window.addEventListener('pointerup', stopResize);
    window.addEventListener('pointercancel', stopResize);
  }
</script>

<div
  class={['app-shell', platformClass, runtimeClass, sidebarCollapsed && 'sidebar-collapsed'].filter(Boolean).join(' ')}
  style:--sidebar-width={`${sidebarWidth}px`}
>
  <header
    class="titlebar"
    role="presentation"
    onpointerdown={startTitlebarDrag}
  >
    <button
      type="button"
      class="sidebar-toggle"
      aria-label={sidebarCollapsed ? '展开侧边栏' : '收起侧边栏'}
      aria-pressed={!sidebarCollapsed}
      onclick={toggleSidebar}
      onpointerdown={(event) => event.stopPropagation()}
    >
      <span class="sidebar-toggle-icon" aria-hidden="true">
        <span></span>
        <span></span>
        <span></span>
      </span>
    </button>
    <div class="titlebar-copy">
      <strong>{shell.title}</strong>
      <span>{shell.subtitle}</span>
    </div>
  </header>

  <aside class="sidebar">
    <div class="sidebar-body">
      <a class="brand" href={resolve('/')}>
        <span class="brand-mark">Cullify</span>
        <span class="brand-ver">v0.1.0</span>
      </a>

      <div class="nav-section">
        <p class="nav-eyebrow">工作区</p>
        {#each navItems as item (item.id)}
          <a class={['nav-item', shell.active === item.id && 'active'].filter(Boolean).join(' ')} href={resolve(item.href)}>
            <span>{item.label}</span>
            {#if item.count}
              <span class="nav-count">{item.count()}</span>
            {/if}
          </a>
        {/each}
      </div>

      {#if shell.sidebarExtra}
        {@render shell.sidebarExtra()}
      {/if}

      {#if shell.showDefaultSidebarDetails}
        <div class="nav-section">
          <p class="nav-eyebrow">最近项目</p>
          {#if shell.recentProjects.length}
            {#each shell.recentProjects.slice(0, 3) as project (project.id)}
              <a class="nav-item" href={resolve('/cull')}>
                <span>{project.shortName}</span>
                <span class="nav-count">{project.total}</span>
              </a>
            {/each}
          {:else}
            <div class="nav-empty">暂无项目</div>
          {/if}
        </div>

        <div class="nav-spacer"></div>

        <p class="nav-eyebrow">推理引擎</p>
        <div class="model-card">
          <div class="model-row">
            <span class="model-name">快速模式</span>
            <span class="model-dot" title="可用"></span>
          </div>
          <div class="model-meta">
            <span>Rust 原生分析</span>
            <span>无需模型</span>
          </div>
          <div class="model-meta">
            <span>模糊 · 曝光 · pHash</span>
            <span>本地</span>
          </div>
        </div>
      {/if}
    </div>
    <button
      type="button"
      class="sidebar-resizer"
      aria-label="调整侧边栏宽度"
      onpointerdown={startSidebarResize}
    ></button>
  </aside>

  {@render children()}

  <footer class="statusbar">
    <div class="side">
      <span class="pip">{shell.footer.photo}</span>
      <span>{shell.footer.analysis}</span>
      {#if shell.task}
        <span class="footer-task">
          {shell.task.label} · {shell.task.detail}
          {#if shell.task.progress !== null}
            <b>{shell.task.progress}%</b>
          {/if}
        </span>
      {/if}
    </div>
    <div class="side">
      <span>{shell.footer.model}</span>
      <span>{cpuLabel}</span>
      <span>{memoryLabel}</span>
      <span>{shell.footer.resources.gpu}</span>
    </div>
  </footer>
</div>
