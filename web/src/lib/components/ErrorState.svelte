<script lang="ts">
	import { Button } from '$lib/components/ui/button/index.js';
	import AlertCircle from '@lucide/svelte/icons/alert-circle';

	let { message, onRetry }: { message: string; onRetry?: () => void } = $props();
</script>

<div class="flex flex-col items-center gap-3 py-12 text-center">
	<AlertCircle class="h-8 w-8 text-destructive" />
	<p class="text-sm text-muted-foreground">{message}</p>
	{#if onRetry}
		<Button variant="outline" size="sm" onclick={onRetry}>Retry</Button>
	{/if}
</div>
