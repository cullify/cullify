<script lang="ts">
  import { resolve } from '$app/paths';
  import { onMount } from 'svelte';
  import {
    defaultAppConfig,
    tryLoadAppConfig,
    trySaveAppConfig,
    type AppConfig
  } from '$lib/backend';
  import { models as seedModels, providers, shortcuts as seedShortcuts } from '$lib/mockData';
  import type { ProviderId, Shortcut, VlmModel } from '$lib/types';

  let activeProvider = $state<ProviderId>(defaultAppConfig.providerId);
  let models = $state<VlmModel[]>(seedModels.map((model) => ({ ...model })));
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

  const activeProviderName = $derived(providers.find((provider) => provider.id === activeProvider)?.name ?? '未配置');
  const activeModelId = $derived(models.find((model) => model.active)?.id ?? defaultAppConfig.activeModelId);
  const dirtyCount = $derived(dirtyFields.length);

  onMount(() => {
    void loadSavedConfig();
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
    activeProvider = normalizeProvider(config.providerId);
    blurThreshold = config.blurThreshold;
    exposureTolerance = config.exposureTolerance;
    cullLine = config.cullLine;
    vlmThreads = config.vlmThreads;
    arenaTarget = config.arenaTarget;
    autoGroup = config.autoGroup;
    gpuMetal = config.gpuMetal;

    models = seedModels.map((model) => ({
      ...model,
      active: model.id === config.activeModelId,
      action: model.id === config.activeModelId ? '已激活' : model.action === '已激活' ? '切换' : model.action
    }));

    shortcuts = seedShortcuts.map((shortcut) => ({
      ...shortcut,
      keys: config.shortcuts.find((item) => item.id === shortcut.id)?.keys ?? [...shortcut.keys]
    }));
    dirtyFields = [];
  }

  function normalizeProvider(providerId: string): ProviderId {
    if (providerId === 'ollama' || providerId === 'openai') return providerId;
    return 'builtin';
  }

  function buildConfig(): AppConfig {
    return {
      providerId: activeProvider,
      activeModelId,
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

  function selectProvider(provider: ProviderId) {
    activeProvider = provider;
    markDirty(`Provider → ${providers.find((item) => item.id === provider)?.name ?? provider}`);
  }

  function activateModel(modelId: string) {
    models = models.map((model) => ({
      ...model,
      active: model.id === modelId,
      action: model.id === modelId ? '已激活' : model.action === '已激活' ? '切换' : model.action
    }));
    markDirty(`模型 → ${models.find((model) => model.id === modelId)?.name ?? modelId}`);
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

<div class="app-shell">
  <aside class="sidebar">
    <a class="brand" href={resolve('/')}>
      <span class="brand-mark">Cullify</span>
      <span class="brand-ver">v0.1.0</span>
    </a>

    <div class="nav-section">
      <p class="nav-eyebrow">导航</p>
      <a class="nav-item" href={resolve('/')}>主控台</a>
      <a class="nav-item" href={resolve('/cull')}>照片挑选</a>
      <a class="nav-item active" href={resolve('/settings')}>设置</a>
    </div>

    <p class="nav-eyebrow">设置分组</p>
    <div class="nav-section">
      <a class="nav-item active" href="#provider">推理引擎 · Provider</a>
      <a class="nav-item" href="#models">VLM 模型管理</a>
      <a class="nav-item" href="#modes">模式参数</a>
      <a class="nav-item" href="#shortcuts">键盘快捷键</a>
      <a class="nav-item" href="#about">数据与关于</a>
    </div>
  </aside>

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
          <h2>推理引擎 · Provider 切换</h2>
          <span>三选一 · 当前为 {activeProviderName}</span>
        </div>
        <p class="section-copy">当前真实分析链路使用快速模式的 Rust 原生质量评分；Provider 配置会被保存，但专家模式要等模型任务队列接入后才参与项目创建。</p>
        <div class="providers">
          {#each providers as provider (provider.id)}
            <button
              type="button"
              class={['provider', activeProvider === provider.id && 'active'].filter(Boolean).join(' ')}
              aria-pressed={activeProvider === provider.id}
              onclick={() => selectProvider(provider.id)}
            >
              <span class="provider-row">
                <strong>{provider.name}</strong>
                <span class={['provider-status', activeProvider === provider.id && 'on'].filter(Boolean).join(' ')}>
                  {activeProvider === provider.id ? '已选择' : provider.status}
                </span>
              </span>
              <span class="provider-desc">{provider.description}</span>
              <span class="provider-meta"><span>{provider.metaLeft}</span><span>{provider.metaRight}</span></span>
            </button>
          {/each}
        </div>
      </section>

      <section id="models" class="section">
        <div class="settings-head">
          <h2>VLM 模型管理</h2>
          <span>当前模型 · {activeModelId}</span>
        </div>
        <p class="section-copy">模型清单为后续专家模式预留。商业版接入 VLM 前，需要把“可下载、可校验、可回滚”作为模型管理的核心能力。</p>
        <div class="model-list">
          {#each models as model (model.id)}
            <div class={['model-row-item', model.active && 'active'].filter(Boolean).join(' ')}>
              <span class="model-state"></span>
              <span>
                <strong>{model.name}</strong>
                <small>{model.file}</small>
              </span>
              <span class="model-size">{model.size}</span>
              <span class="model-speed">{model.speed}</span>
              <button type="button" class="model-action" onclick={() => activateModel(model.id)}>{model.action}</button>
            </div>
          {/each}
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
  <footer class="statusbar">
    <div class="side">
      <span class="pip">引擎 · {activeProviderName}</span>
      <span>{configPath}</span>
    </div>
    <div class="side">
      <span>{dirtyCount ? `${dirtyCount} 项未保存` : '配置已保存'}</span>
      <span>本地版</span>
    </div>
  </footer>
</div>

<style>
  .settings-wrap {
    max-width: 920px;
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

  .providers {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
  }

  .provider {
    display: flex;
    min-height: 154px;
    flex-direction: column;
    gap: 8px;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--surface);
    padding: 16px;
    text-align: left;
  }

  .provider.active {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent);
  }

  .provider-row,
  .provider-meta {
    display: flex;
    justify-content: space-between;
    gap: 12px;
  }

  .provider-row strong {
    font-family: var(--font-display);
    font-size: 15px;
    font-weight: 500;
  }

  .provider-status {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 10px;
    text-transform: uppercase;
  }

  .provider-status::before {
    width: 6px;
    height: 6px;
    border-radius: 50%;
    background: var(--meta);
    content: "";
  }

  .provider-status.on::before {
    background: var(--success);
  }

  .provider-desc {
    flex: 1;
    color: var(--muted);
    font-size: 12px;
    line-height: 1.55;
  }

  .provider-meta {
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 10px;
  }

  .model-list {
    overflow: hidden;
    border: 1px solid var(--border);
    border-radius: 8px;
  }

  .model-row-item {
    display: grid;
    grid-template-columns: 24px 1fr auto auto auto;
    align-items: center;
    gap: 14px;
    border-bottom: 1px solid var(--border-soft);
    background: var(--surface);
    padding: 14px 16px;
  }

  .model-row-item:last-child {
    border-bottom: 0;
  }

  .model-row-item.active {
    background: var(--tag-bg-faint);
  }

  .model-state {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--border);
  }

  .model-row-item.active .model-state {
    background: var(--success);
  }

  .model-row-item strong {
    display: block;
    font-family: var(--font-display);
    font-size: 15px;
    font-weight: 500;
  }

  .model-row-item small {
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 10px;
  }

  .model-size,
  .model-speed,
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

  .model-row-item.active .model-action {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--accent-on);
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

  select {
    min-width: 160px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--surface);
    color: var(--fg);
    padding: 7px 12px;
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
    .providers {
      grid-template-columns: 1fr;
    }

    .model-row-item,
    .form-row {
      grid-template-columns: 1fr;
      align-items: start;
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
