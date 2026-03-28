<script lang="ts">
	import Inbox from '@lucide/svelte/icons/inbox';

	let { message = 'No data' }: { message?: string } = $props();
</script>

<div class="flex flex-col items-center gap-2 py-12 text-center">
	<Inbox class="h-8 w-8 text-muted-foreground" />
	<p class="text-sm text-muted-foreground">{message}</p>
</div>
