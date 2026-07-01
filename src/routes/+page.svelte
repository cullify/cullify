<script lang="ts">
  import { resolve } from '$app/paths';
  import { onMount } from 'svelte';
  import AppShell from '$lib/components/AppShell.svelte';
  import {
    backendProjectToProject,
    createProjectFromPickedFolderWithOutcome,
    defaultAppConfig,
    deleteProject,
    renameProject,
    tryLoadAppConfig,
    tryListProjects
  } from '$lib/backend';
  import { projects as mockProjects, shortcuts } from '$lib/mockData';
  import type { CullMode, Project, Shortcut } from '$lib/types';

  type ModeCard = {
    id: CullMode;
    label: string;
    name: string;
    desc: string;
    left: string;
    right: string;
    status: string;
    available: boolean;
    unavailableMessage?: string;
  };

  const modeCards: ModeCard[] = [
    {
      id: 'quick',
      label: '— 快速',
      name: 'Rust 原生引擎',
      desc: '零配置 · Laplacian 模糊 + 直方图曝光检测。适合先完成整组照片的本地快速初筛。',
      left: '~ 1 ms/张',
      right: '无需模型',
      status: '当前可用',
      available: true
    },
    {
      id: 'expert',
      label: '— 专家',
      name: 'VLM 深度分析',
      desc: '面向人脸 / 构图 / 闭眼 / 语义描述的增强链路。Provider 与模型任务队列接入后开放创建。',
      left: '~ 2.4 s/张',
      right: 'Provider 可切换',
      status: '接入中',
      available: false,
      unavailableMessage: '专家模式还未开放真实 VLM 分析链路；当前请使用快速模式完成本地选片。'
    },
    {
      id: 'arena',
      label: '— 竞技场',
      name: 'A / B 键盘 PK',
      desc: '质量分相近的照片两两配对，A / D 键选择胜者。配对与晋级状态机完成后开放创建。',
      left: 'Tournament',
      right: '支持撤销',
      status: '规划中',
      available: false,
      unavailableMessage: '竞技场模式依赖配对与晋级状态机；当前请使用快速模式完成初筛与导出。'
    }
  ];

  const tagClass = (mode: string) => `tag tag-${mode}`;
  const statusClass = (status: string) => `project-status ${status}`;

  let projects = $state<Project[]>(mockProjects);
  let shortcutRows = $state<Shortcut[]>(shortcuts.map((shortcut) => ({ ...shortcut, keys: [...shortcut.keys] })));
  let isDemoData = $state(true);
  let isCreating = $state(false);
  let createMessage = $state('');
  let createMessageTone = $state<'info' | 'success' | 'error'>('info');
  let appDataDir = $state('桌面环境可用');

  const totalProcessed = $derived(projects.reduce((total, project) => total + project.total, 0));
  const totalCulled = $derived(projects.reduce((total, project) => total + project.culled, 0));
  const cullRatio = $derived(totalProcessed ? Math.round((totalCulled / totalProcessed) * 100) : 0);
  const statusSummary = $derived(isDemoData ? '浏览器演示数据' : `${projects.length} 个本地项目`);
  const databasePath = $derived(appDataDir === '桌面环境可用' ? '桌面数据目录' : `${appDataDir}/cullify.db`);

  onMount(() => {
    void loadShortcuts();
    void refreshProjects();
  });

  async function refreshProjects() {
    const backendProjects = await tryListProjects();
    if (backendProjects !== null) {
      projects = backendProjects.map(backendProjectToProject);
      isDemoData = false;
    }
  }

  async function loadShortcuts() {
    const envelope = await tryLoadAppConfig();
    const configuredShortcuts = envelope?.config.shortcuts ?? defaultAppConfig.shortcuts;
    appDataDir = envelope?.appDataDir ?? '桌面环境可用';
    shortcutRows = shortcuts.map((shortcut) => ({
      ...shortcut,
      keys: configuredShortcuts.find((item) => item.id === shortcut.id)?.keys ?? [...shortcut.keys]
    }));
  }

  async function createFromFolder(mode: CullMode = 'quick') {
    if (mode !== 'quick') {
      showModeUnavailable(mode);
      return;
    }

    if (isCreating) return;
    isCreating = true;
    createMessageTone = 'info';
    createMessage = '正在扫描文件夹并生成本地质量评分...';

    try {
      const outcome = await createProjectFromPickedFolderWithOutcome(mode);
      if (outcome.status === 'created') {
        const { summary } = outcome;
        createMessageTone = 'success';
        createMessage = `已导入 ${summary.inserted.toLocaleString()} 张照片，跳过 ${summary.skipped.toLocaleString()} 个文件，正在进入挑选页...`;
        localStorage.setItem('cullify.activeProjectId', outcome.summary.project.id);
        await refreshProjects();
        window.location.href = resolve('/cull');
      } else if (outcome.status === 'cancelled') {
        createMessageTone = 'info';
        createMessage = '已取消选择文件夹。';
      } else {
        createMessageTone = 'error';
        createMessage = describeCreateError(outcome.error);
      }
    } finally {
      isCreating = false;
    }
  }

  async function chooseMode(mode: ModeCard) {
    if (!mode.available) {
      createMessageTone = 'info';
      createMessage = mode.unavailableMessage ?? '该模式仍在接入中；当前请使用快速模式完成本地选片。';
      return;
    }

    await createFromFolder(mode.id);
  }

  function showModeUnavailable(mode: CullMode) {
    const modeCard = modeCards.find((item) => item.id === mode);
    createMessageTone = 'info';
    createMessage = modeCard?.unavailableMessage ?? '该模式仍在接入中；当前请使用快速模式完成本地选片。';
  }

  function openProject(project: Project) {
    localStorage.setItem('cullify.activeProjectId', project.id);
  }

  async function renameExistingProject(project: Project) {
    if (isCreating) return;
    if (!project.backend) {
      createMessageTone = 'info';
      createMessage = '浏览器演示项目不可修改；请在 Tauri 桌面应用中管理真实项目。';
      return;
    }

    const nextName = window.prompt('重命名项目', project.name)?.trim();
    if (!nextName || nextName === project.name) return;

    try {
      await renameProject(project.id, nextName);
      createMessageTone = 'success';
      createMessage = `项目已重命名为「${nextName}」。`;
      await refreshProjects();
    } catch (error) {
      createMessageTone = 'error';
      createMessage = describeProjectActionError(error);
    }
  }

  async function deleteExistingProject(project: Project) {
    if (isCreating) return;
    if (!project.backend) {
      createMessageTone = 'info';
      createMessage = '浏览器演示项目不可删除；请在 Tauri 桌面应用中管理真实项目。';
      return;
    }

    const confirmed = window.confirm(`删除「${project.name}」？\n\n只会删除 Cullify 项目记录、决策和缩略图缓存，不会删除原始照片。`);
    if (!confirmed) return;

    try {
      await deleteProject(project.id);
      if (localStorage.getItem('cullify.activeProjectId') === project.id) {
        localStorage.removeItem('cullify.activeProjectId');
      }
      createMessageTone = 'success';
      createMessage = `已删除「${project.name}」，原始照片未受影响。`;
      await refreshProjects();
    } catch (error) {
      createMessageTone = 'error';
      createMessage = describeProjectActionError(error);
    }
  }

  function describeCreateError(error: string) {
    if (error.includes('no supported images')) {
      return '未找到支持的图片文件。请导入包含 JPG、PNG、WebP 或 TIFF 的文件夹。';
    }

    if (error.includes('does not exist') || error.includes('not a directory')) {
      return '选择的路径不是可读取的文件夹，请重新选择照片目录。';
    }

    if (
      error.includes('__TAURI__') ||
      error.includes('invoke') ||
      error.includes('not available')
    ) {
      return '导入真实照片需要在 Tauri 桌面环境中运行。当前浏览器预览会保留演示数据。';
    }

    return `导入失败：${error}`;
  }

  function describeProjectActionError(error: unknown) {
    const message = error instanceof Error ? error.message : typeof error === 'string' ? error : 'unknown error';
    if (message.includes('project name cannot be empty')) return '项目名称不能为空。';
    if (message.includes('project not found')) return '项目不存在，可能已被删除。';
    if (message.includes('__TAURI__') || message.includes('invoke') || message.includes('not available')) {
      return '项目管理需要在 Tauri 桌面环境中运行。';
    }
    return `项目操作失败：${message}`;
  }
</script>

<AppShell
  active="dashboard"
  recentProjects={projects}
  cullCount={String(totalProcessed)}
  statusLeft={['引擎就绪 · 快速模式可用', statusSummary, '数据 · 本机']}
  statusRight={[databasePath, '本地版']}
>
  <main class="app-main scroll">
    <div class="page-pad">
      <header class="topbar">
        <div>
          <h1 class="page-title">主控台</h1>
          <p class="page-lede">本地离线 AI 照片选片 · 数据不出本机</p>
        </div>
        <div class="actions">
          <button class="btn btn-secondary" type="button" onclick={() => createFromFolder('quick')} disabled={isCreating}>
            打开文件夹
          </button>
          <button class="btn btn-primary" type="button" onclick={() => createFromFolder('quick')} disabled={isCreating}>
            {isCreating ? '扫描中' : '新建项目'}
          </button>
        </div>
      </header>

      {#if createMessage}
        <div class={`import-note ${createMessageTone}`} role="status">{createMessage}</div>
      {/if}

      <section>
        <div class="section-head">
          <div>
            <p class="section-num">01</p>
            <h2 class="section-title">选择工作模式</h2>
            <p class="section-lede">当前生产可用链路为快速模式；专家与竞技场按原型保留入口状态</p>
          </div>
        </div>

        <div class="modes">
          {#each modeCards as mode (mode.id)}
            <button
              class={['mode-card', !mode.available && 'mode-card-planned'].filter(Boolean).join(' ')}
              type="button"
              onclick={() => chooseMode(mode)}
              disabled={isCreating}
              aria-describedby={`${mode.id}-mode-desc`}
            >
              <span class="mode-head">
                <span class="mode-num">{mode.label}</span>
                <span class={['mode-status', mode.available ? 'ready' : 'planned'].join(' ')}>{mode.status}</span>
              </span>
              <span class="mode-name">{mode.name}</span>
              <p id={`${mode.id}-mode-desc`}>{mode.desc}</p>
              <span class="mode-foot">
                <span>{mode.left}</span>
                <span>{mode.right}</span>
              </span>
            </button>
          {/each}
        </div>
      </section>

      <section class="metrics" aria-label="累计指标">
        <div>
          <strong>{totalProcessed.toLocaleString()}</strong>
          <span>累计处理照片</span>
        </div>
        <div>
          <strong>{totalCulled.toLocaleString()}</strong>
          <span>累计淘汰 · {cullRatio}%</span>
        </div>
        <div>
          <strong>本地</strong>
          <span>快速评分引擎</span>
        </div>
        <div>
          <strong>离线</strong>
          <span>照片不出本机</span>
        </div>
      </section>

      <section>
        <div class="section-head">
          <div>
            <p class="section-num">02</p>
            <h2 class="section-title">最近项目</h2>
            <p class="section-lede">点击任意行进入照片挑选界面</p>
          </div>
          <a class="section-link" href={resolve('/cull')}>查看全部</a>
        </div>

        {#if projects.length}
          <div class="projects">
            {#each projects as project (project.id)}
              <div class="project-row">
                <a class="project-name-link" href={resolve('/cull')} onclick={() => openProject(project)}>
                  <strong>{project.name}</strong>
                  <small>{project.path}</small>
                </a>
                <span><span class={tagClass(project.mode)}>{project.mode}</span></span>
                <span class="col strong project-total">{project.total.toLocaleString()}</span>
                <span class="col project-kept">保留 {project.kept || '—'}</span>
                <span class="col muted project-cull">淘汰 {project.culled || '—'}</span>
                <span class="col project-state"><span class={statusClass(project.status)}>{project.statusLabel}</span></span>
                <span class="project-actions">
                  <button type="button" onclick={() => renameExistingProject(project)} disabled={isCreating}>
                    重命名
                  </button>
                  <button type="button" class="danger" onclick={() => deleteExistingProject(project)} disabled={isCreating}>
                    删除
                  </button>
                </span>
              </div>
            {/each}
          </div>
        {:else}
          <div class="project-empty">
            <strong>还没有本地项目</strong>
            <span>点击“新建项目”选择照片文件夹，Cullify 会在本机扫描、评分并生成可导出的选片项目。</span>
            <button class="btn btn-secondary" type="button" onclick={() => createFromFolder('quick')} disabled={isCreating}>
              选择照片文件夹
            </button>
          </div>
        {/if}
      </section>

      <section class="shortcut-section">
        <div class="section-head">
          <div>
            <p class="section-num">03</p>
            <h2 class="section-title">键盘速查</h2>
            <p class="section-lede">挑选界面 · 全键盘操作</p>
          </div>
        </div>

        <div class="shortcuts">
          {#each shortcutRows.slice(0, 8) as shortcut (shortcut.id)}
            <div class="shortcut-row">
              <span>{shortcut.action}</span>
              <span class="keys">
                {#each shortcut.keys as key (`${shortcut.id}-${key}`)}
                  <span class="kbd">{key}</span>
                {/each}
              </span>
            </div>
          {/each}
        </div>
      </section>
    </div>
  </main>
</AppShell>

<style>
  .actions {
    display: flex;
    gap: 8px;
  }

  .import-note {
    margin: -8px 0 28px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    color: var(--muted);
    font-size: 13px;
    line-height: 1.45;
    padding: 10px 12px;
  }

  .import-note.success {
    border-color: color-mix(in srgb, var(--success) 55%, var(--border));
    color: var(--success);
  }

  .import-note.error {
    border-color: color-mix(in srgb, var(--danger) 55%, var(--border));
    color: var(--danger);
  }

  .modes {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 16px;
    margin-bottom: 36px;
  }

  .mode-card {
    display: flex;
    min-height: 154px;
    flex-direction: column;
    gap: 10px;
    color: inherit;
    border: 1px solid var(--border);
    border-radius: 12px;
    background: var(--surface);
    padding: 20px 20px 18px;
    text-align: left;
    transition: box-shadow 0.2s ease;
  }

  .mode-card:hover {
    box-shadow: var(--shadow-soft);
  }

  .mode-card-planned {
    background:
      linear-gradient(135deg, color-mix(in srgb, var(--surface) 88%, var(--bg)) 0%, var(--surface) 100%);
  }

  .mode-card-planned:hover {
    box-shadow: 0 0 0 1px var(--border);
  }

  .mode-card:disabled {
    cursor: wait;
    opacity: 0.62;
  }

  .mode-head,
  .mode-status {
    display: flex;
    align-items: center;
  }

  .mode-head {
    justify-content: space-between;
    gap: 12px;
  }

  .mode-status {
    border: 1px solid var(--border);
    border-radius: 999px;
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 10px;
    line-height: 1;
    padding: 4px 7px;
  }

  .mode-status.ready {
    border-color: color-mix(in srgb, var(--success) 45%, var(--border));
    color: var(--success);
  }

  .mode-status.planned {
    border-color: var(--border-soft);
    color: var(--meta);
  }

  .mode-num,
  .mode-foot,
  .section-link {
    color: var(--accent);
    font-family: var(--font-mono);
    font-size: 11px;
    letter-spacing: 0.5px;
  }

  .mode-name {
    color: var(--fg);
    font-family: var(--font-display);
    font-size: 20px;
    font-weight: 500;
    line-height: 1.2;
  }

  .mode-card p {
    flex: 1;
    margin: 0;
    color: var(--muted);
    font-size: 13px;
    line-height: 1.55;
  }

  .mode-foot {
    display: flex;
    justify-content: space-between;
    border-top: 1px solid var(--border-soft);
    color: var(--meta);
    padding-top: 8px;
  }

  .metrics {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    margin: 32px 0;
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
    padding: 18px 0;
  }

  .metrics div {
    border-right: 1px solid var(--border-soft);
    padding: 0 20px;
  }

  .metrics div:last-child {
    border-right: 0;
  }

  .metrics strong {
    display: block;
    color: var(--accent);
    font-family: var(--font-display);
    font-size: 28px;
    font-variant-numeric: tabular-nums;
    font-weight: 500;
    line-height: 1.1;
  }

  .metrics span {
    display: block;
    margin-top: 6px;
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 10px;
    letter-spacing: 0.9px;
    text-transform: uppercase;
  }

  .projects {
    display: flex;
    flex-direction: column;
    border-top: 1px solid var(--border);
  }

  .project-empty {
    display: flex;
    min-height: 154px;
    flex-direction: column;
    align-items: flex-start;
    justify-content: center;
    gap: 10px;
    border: 1px dashed var(--border);
    border-radius: 8px;
    background: var(--surface);
    color: var(--muted);
    font-size: 13px;
    line-height: 1.55;
    padding: 22px;
  }

  .project-empty strong {
    color: var(--fg);
    font-family: var(--font-display);
    font-size: 18px;
    font-weight: 500;
  }

  .project-empty span {
    max-width: 58ch;
  }

  .project-row {
    display: grid;
    grid-template-columns: 1.4fr 80px 100px 100px 100px 110px 132px;
    align-items: center;
    gap: 16px;
    border-bottom: 1px solid var(--border-soft);
    padding: 14px 4px;
    transition: background 0.12s ease;
  }

  .project-row:hover {
    background: var(--surface);
  }

  .project-name-link {
    min-width: 0;
  }

  .project-row strong {
    display: block;
    color: var(--fg);
    font-family: var(--font-display);
    font-size: 15px;
    font-weight: 500;
  }

  .project-row small {
    display: block;
    overflow: hidden;
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 11px;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .project-actions {
    display: flex;
    justify-content: flex-end;
    gap: 6px;
  }

  .project-actions button {
    border: 1px solid var(--border);
    border-radius: 5px;
    background: var(--surface);
    color: var(--accent);
    font-size: 12px;
    font-weight: 700;
    padding: 5px 8px;
  }

  .project-actions button:hover {
    background: var(--surface-warm);
  }

  .project-actions button.danger {
    color: var(--danger);
  }

  .project-actions button:disabled {
    cursor: wait;
    opacity: 0.55;
  }

  .col {
    color: var(--fg-2);
    font-family: var(--font-mono);
    font-size: 12px;
    text-align: right;
  }

  .strong {
    font-weight: 700;
  }

  .project-status {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    color: var(--meta);
  }

  .project-status::before {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--meta);
    content: "";
  }

  .project-status.done::before {
    background: var(--success);
  }

  .project-status.running::before {
    background: var(--accent);
  }

  .project-status.paused::before {
    background: var(--warn);
  }

  .shortcut-section {
    margin-top: 36px;
  }

  .shortcuts {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 12px 24px;
  }

  .shortcut-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 12px;
    border-bottom: 1px solid var(--border-soft);
    color: var(--muted);
    font-size: 12px;
    padding: 6px 0;
  }

  .keys {
    display: flex;
    gap: 4px;
  }

  @media (max-width: 1100px) {
    .modes {
      grid-template-columns: 1fr;
    }

    .metrics,
    .shortcuts {
      grid-template-columns: repeat(2, 1fr);
    }

    .project-row {
      grid-template-columns: 1.3fr 80px 80px 96px 118px;
    }

    .project-kept,
    .project-cull {
      display: none;
    }
  }

  @media (max-width: 760px) {
    .topbar,
    .section-head {
      align-items: flex-start;
      flex-direction: column;
    }

    .actions,
    .metrics,
    .shortcuts {
      grid-template-columns: 1fr;
      width: 100%;
    }

    .actions {
      display: grid;
    }

    .project-row {
      grid-template-columns: 1fr;
      gap: 8px;
    }

    .col {
      text-align: left;
    }

    .project-actions {
      justify-content: flex-start;
    }
  }
</style>
