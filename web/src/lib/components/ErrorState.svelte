<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import AlertCircle from '@lucide/svelte/icons/alert-circle';
	import ShieldX from '@lucide/svelte/icons/shield-x';

	let { message, onRetry }: { message: string; onRetry?: () => void } = $props();

	const isPermissionError = $derived(
		/permission|forbidden|requires.*role/i.test(message)
	);
</script>

{#if isPermissionError}
	<div class="flex flex-col items-center gap-4 py-16 text-center">
		<div class="rounded-full bg-muted p-4">
			<ShieldX class="h-10 w-10 text-muted-foreground" />
		</div>
		<div class="space-y-1">
			<h2 class="text-lg font-semibold">Access denied</h2>
			<p class="text-sm text-muted-foreground max-w-sm">You don't have permission to view this page. Contact your organization admin to request access.</p>
		</div>
	</div>
{:else}
	<div class="flex flex-col items-center gap-3 py-12 text-center">
		<AlertCircle class="h-8 w-8 text-destructive" />
		<p class="text-sm text-muted-foreground">{message}</p>
		{#if onRetry}
			<Button variant="outline" size="sm" onclick={onRetry}>Retry</Button>
		{/if}
	</div>
{/if}
