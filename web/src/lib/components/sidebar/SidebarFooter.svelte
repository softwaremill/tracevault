<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { LogOut, ChevronsRight } from '@lucide/svelte';

	let {
		expanded,
		userEmail,
		onLogout,
		onExpand
	}: {
		expanded: boolean;
		userEmail: string | null;
		onLogout: () => void;
		onExpand: () => void;
	} = $props();
</script>

<div class="border-t p-2">
	{#if !expanded}
		<button
			onclick={onExpand}
			class="flex h-10 w-10 items-center justify-center rounded-md text-muted-foreground hover:text-foreground hover:bg-sidebar-accent mx-auto"
		>
			<ChevronsRight class="h-4 w-4" />
		</button>
	{:else}
		{#if userEmail}
			<p class="text-xs text-muted-foreground truncate px-3 py-1">{userEmail}</p>
		{/if}
		<Button variant="ghost" size="sm" class="w-full justify-start gap-2" onclick={onLogout}>
			<LogOut class="h-4 w-4" />
			Log out
		</Button>
	{/if}
</div>
