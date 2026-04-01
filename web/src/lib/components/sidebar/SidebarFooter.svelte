<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import { LogOut } from '@lucide/svelte';

	let {
		expanded,
		userEmail,
		onLogout
	}: {
		expanded: boolean;
		userEmail: string | null;
		onLogout: () => void;
	} = $props();
</script>

{#if expanded}
	<div class="border-t p-2">
		{#if userEmail}
			<p class="text-xs text-muted-foreground truncate px-3 py-1">{userEmail}</p>
		{/if}
		<Button variant="ghost" size="sm" class="w-full justify-start gap-2" onclick={onLogout}>
			<LogOut class="h-4 w-4" />
			Log out
		</Button>
	</div>
{/if}
