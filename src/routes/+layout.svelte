<script lang="ts">
    import '../app.css';
    import AppShell from '$lib/components/AppShell.svelte';
    import {ShellController, setShellContext} from '$lib/shell.svelte';
    import {ModeWatcher} from "mode-watcher";

    let {children} = $props();
    const shell = new ShellController();
    setShellContext(shell);
</script>

<AppShell {shell}>
    <ModeWatcher/>
    {@render children()}
</AppShell>
