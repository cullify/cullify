<script lang="ts">
  import '../app.css';
  import AppShell from '$lib/components/AppShell.svelte';
  import { ShellController, setShellContext } from '$lib/shell.svelte';

  let { children } = $props();
  const shell = new ShellController();
  setShellContext(shell);
</script>

<AppShell {shell}>
  {@render children()}
</AppShell>
