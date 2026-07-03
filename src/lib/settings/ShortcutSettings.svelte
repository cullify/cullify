<script lang="ts">
  import { Button } from '$lib/components/ui/button';
  import { Kbd } from '$lib/components/ui/kbd';
  import * as Table from '$lib/components/ui/table';
  import type { Shortcut } from '$lib/types';

  interface Props {
    shortcuts: Shortcut[];
    editingShortcut: string | null;
    setEditingShortcut: (shortcutId: string) => void;
    remapShortcut: (event: KeyboardEvent, shortcut: Shortcut) => void;
  }

  let { shortcuts, editingShortcut, setEditingShortcut, remapShortcut }: Props = $props();
</script>

<section id="shortcuts" class="section">
  <div class="settings-head">
    <h2>键盘快捷键</h2>
    <span>点击按键即可重映射 · 文本框中自动暂停</span>
  </div>
  <p class="section-copy">摄影师的肌肉记忆比工具默认值重要。点击任意快捷键单元即可重映射，保存后挑选页会按本地配置执行。</p>
  <Table.Root class="shortcuts-table">
    <Table.Header>
      <Table.Row><Table.Head>动作</Table.Head><Table.Head>场景</Table.Head><Table.Head>快捷键</Table.Head></Table.Row>
    </Table.Header>
    <Table.Body>
      {#each shortcuts as shortcut (shortcut.id)}
        <Table.Row>
          <Table.Cell>{shortcut.action}</Table.Cell>
          <Table.Cell>{shortcut.scenario}</Table.Cell>
          <Table.Cell class="keys">
            <Button
              variant="ghost"
              size="sm"
              class={['kbd-cell', editingShortcut === shortcut.id && 'editing'].filter(Boolean).join(' ')}
              onkeydown={(event) => remapShortcut(event, shortcut)}
              onclick={() => setEditingShortcut(shortcut.id)}
            >
              {#each shortcut.keys as key (`${shortcut.id}-${key}`)}
                <Kbd class="kbd">{key}</Kbd>
              {/each}
              <span class="hint">{editingShortcut === shortcut.id ? '按键...' : '编辑'}</span>
            </Button>
          </Table.Cell>
        </Table.Row>
      {/each}
    </Table.Body>
  </Table.Root>
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

  :global(.shortcuts-table) {
    width: 100%;
    border-collapse: collapse;
  }

  :global(.shortcuts-table th),
  :global(.shortcuts-table td) {
    border-bottom: 1px solid var(--border-soft);
    padding: 10px 14px;
    text-align: left;
  }

  :global(.shortcuts-table th) {
    border-bottom-color: var(--border);
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 500;
    letter-spacing: 1.1px;
    text-transform: uppercase;
  }

  :global(.shortcuts-table td:first-child) {
    color: var(--fg);
    font-family: var(--font-display);
    font-weight: 500;
  }

  :global(.shortcuts-table td:nth-child(2)) {
    color: var(--muted-foreground);
    font-size: 12px;
  }

  :global(.keys) {
    text-align: right;
  }

  :global(.kbd-cell) {
    display: inline-flex;
    align-items: center;
    gap: 3px;
    border-radius: 4px;
    background: transparent;
    padding: 4px 8px;
  }

  :global(.kbd-cell:hover),
  :global(.kbd-cell.editing) {
    background: var(--tag-bg-soft);
  }

  .hint {
    margin-left: 6px;
    color: var(--meta);
    font-family: var(--font-mono);
    font-size: 9px;
    text-transform: uppercase;
  }

  @media (max-width: 1000px) {
    .settings-head {
      align-items: flex-start;
      flex-direction: column;
    }
  }
</style>
