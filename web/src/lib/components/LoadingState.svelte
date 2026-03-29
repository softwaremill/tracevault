<script lang="ts">
	import { Skeleton } from '$lib/components/ui/skeleton/index.js';

	let { lines = 5 }: { lines?: number } = $props();
</script>

<div class="space-y-3 p-4">
	{#each Array(lines) as _}
		<Skeleton class="h-4 w-full" />
	{/each}
</div>
