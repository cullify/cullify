<script lang="ts">
  import { resolve } from '$app/paths';
  import type { Snippet } from 'svelte';
  import type { AppRoute, Project } from '$lib/types';

  interface Props {
    active: AppRoute;
    children: Snippet;
    inspector?: Snippet;
    sidebarExtra?: Snippet;
    showDefaultSidebarDetails?: boolean;
    recentProjects?: Project[];
    cullCount?: string;
    statusLeft?: string[];
    statusRight?: string[];
  }

  let {
    active,
    children,
    inspector,
    sidebarExtra,
    showDefaultSidebarDetails = true,
    recentProjects = [],
    cullCount = '0',
    statusLeft = ['引擎就绪 · 快速模式可用', '模型 · 未强制要求', '数据 · 本机'],
    statusRight = ['~/.cullify/cullify.db', '本地版']
  }: Props = $props();

  type RoutePath = '/' | '/cull' | '/settings';

  const navItems: Array<{ id: AppRoute; label: string; href: RoutePath; count?: () => string }> = [
    { id: 'dashboard', label: '主控台', href: '/', count: () => '·' },
    { id: 'cull', label: '照片挑选', href: '/cull', count: () => cullCount },
    { id: 'settings', label: '设置', href: '/settings', count: () => '·' }
  ];
</script>

<div class={['app-shell', inspector && 'with-inspector'].filter(Boolean).join(' ')}>
  <aside class="sidebar">
    <a class="brand" href={resolve('/')}>
      <span class="brand-mark">Cullify</span>
      <span class="brand-ver">v0.1.0</span>
    </a>

    <div class="nav-section">
      <p class="nav-eyebrow">工作区</p>
      {#each navItems as item (item.id)}
        <a class={['nav-item', active === item.id && 'active'].filter(Boolean).join(' ')} href={resolve(item.href)}>
          <span>{item.label}</span>
          {#if item.count}
            <span class="nav-count">{item.count()}</span>
          {/if}
        </a>
      {/each}
    </div>

    {#if sidebarExtra}
      {@render sidebarExtra()}
    {/if}

    {#if showDefaultSidebarDetails}
      <div class="nav-section">
        <p class="nav-eyebrow">最近项目</p>
        {#if recentProjects.length}
          {#each recentProjects.slice(0, 3) as project (project.id)}
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
  </aside>

  {@render children()}

  {#if inspector}
    <aside class="inspector">
      {@render inspector()}
    </aside>
  {/if}

  <footer class="statusbar">
    <div class="side">
      {#each statusLeft as item, index (`left-${index}`)}
        <span class={index === 0 ? 'pip' : ''}>{item}</span>
      {/each}
    </div>
    <div class="side">
      {#each statusRight as item, index (`right-${index}`)}
        <span>{item}</span>
      {/each}
    </div>
  </footer>
</div>
