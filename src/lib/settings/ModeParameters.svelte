<script lang="ts">
  import { Input } from '$lib/components/ui/input';
  import { NativeSelect, NativeSelectOption } from '$lib/components/ui/native-select';
  import { Switch } from '$lib/components/ui/switch';
  import type { NumberSettingField } from '$lib/settings/modelHelpers';

  interface Props {
    blurThreshold: number;
    exposureTolerance: number;
    cullLine: number;
    vlmThreads: number;
    arenaTarget: string;
    autoGroup: boolean;
    gpuMetal: boolean;
    updateNumber: (field: NumberSettingField, value: number) => void;
    setArenaTarget: (value: string) => void;
    toggleAutoGroup: () => void;
    toggleGpuMetal: () => void;
  }

  let {
    blurThreshold,
    exposureTolerance,
    cullLine,
    vlmThreads,
    arenaTarget,
    autoGroup,
    gpuMetal,
    updateNumber,
    setArenaTarget,
    toggleAutoGroup,
    toggleGpuMetal
  }: Props = $props();
</script>

<section id="modes" class="section">
  <div class="settings-head">
    <h2>模式参数</h2>
    <span>阈值决定自动淘汰的激进程度</span>
  </div>
  <p class="section-copy">快速模式会读取模糊、曝光、淘汰线和连拍分组参数；竞技场与 VLM 线程参数会先保存为后续能力配置。</p>

  <div class="form-row">
    <span><strong>模糊检测阈值</strong><small>Laplacian 方差低于此值判定模糊</small></span>
    <label class="range-control">
      <Input
        type="range"
        min="0"
        max="300"
        step="5"
        value={blurThreshold}
        oninput={(event) => updateNumber('blur', Number(event.currentTarget.value))}
      />
      <span>{blurThreshold}.0</span>
    </label>
    <span class="form-value">默认 100</span>
  </div>

  <div class="form-row">
    <span><strong>曝光宽容度</strong><small>直方图两端裁切比例</small></span>
    <label class="range-control">
      <Input
        type="range"
        min="0"
        max="0.1"
        step="0.001"
        value={exposureTolerance}
        oninput={(event) => updateNumber('exposure', Number(event.currentTarget.value))}
      />
      <span>{exposureTolerance.toFixed(3)}</span>
    </label>
    <span class="form-value">0.018</span>
  </div>

  <div class="form-row">
    <span><strong>质量分淘汰线</strong><small>低于此值自动进入淘汰池</small></span>
    <label class="range-control">
      <Input
        type="range"
        min="0"
        max="100"
        step="1"
        value={cullLine}
        oninput={(event) => updateNumber('cull', Number(event.currentTarget.value))}
      />
      <span>{cullLine} / 100</span>
    </label>
    <span class="form-value">自动模式</span>
  </div>

  <div class="form-row">
    <span><strong>竞技场目标保留</strong><small>循环 PK 直到剩余此比例</small></span>
    <NativeSelect value={arenaTarget} onchange={(event) => setArenaTarget(event.currentTarget.value)}>
      <NativeSelectOption>10%</NativeSelectOption>
      <NativeSelectOption>20%</NativeSelectOption>
      <NativeSelectOption>30%</NativeSelectOption>
      <NativeSelectOption>50%</NativeSelectOption>
    </NativeSelect>
    <span class="form-value">约 250 张</span>
  </div>

  <div class="form-row">
    <span><strong>并发推理线程</strong><small>Semaphore 同时进行的 VLM 调用数</small></span>
    <label class="range-control">
      <Input
        type="range"
        min="1"
        max="8"
        step="1"
        value={vlmThreads}
        oninput={(event) => updateNumber('threads', Number(event.currentTarget.value))}
      />
      <span>{vlmThreads}</span>
    </label>
    <span class="form-value">最大 8</span>
  </div>

  <div class="form-row">
    <span><strong>连拍自动分组</strong><small>基于 pHash 相似度自动聚合连拍</small></span>
    <Switch aria-label="切换连拍自动分组" checked={autoGroup} onclick={toggleAutoGroup} />
    <span class="form-value">{autoGroup ? '已开启' : '已关闭'}</span>
  </div>

  <div class="form-row">
    <span><strong>GPU 加速 · Metal</strong><small>使用 Apple GPU 进行推理加速</small></span>
    <Switch aria-label="切换 Metal GPU 加速" checked={gpuMetal} onclick={toggleGpuMetal} />
    <span class="form-value">{gpuMetal ? '已开启' : '已关闭'}</span>
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

  .settings-head span,
  .section-copy {
    color: var(--muted-foreground);
    font-size: 12px;
  }

  .section-copy {
    max-width: 72ch;
    margin: -8px 0 18px;
    line-height: 1.65;
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

  .range-control :global(input[type='range']) {
    width: 100%;
    accent-color: var(--accent);
  }

  .range-control span,
  .form-value {
    color: var(--fg-2);
    font-family: var(--font-mono);
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    white-space: pre-line;
  }

  @media (max-width: 1000px) {
    .form-row {
      grid-template-columns: 1fr;
      align-items: start;
    }

    .settings-head {
      align-items: flex-start;
      flex-direction: column;
    }
  }
</style>
