<script lang="ts">
	import AnalyticsFilters from '$lib/components/analytics-filters.svelte';
	let { children } = $props();
</script>

<div class="space-y-4">
	<AnalyticsFilters />
	{@render children()}
</div>
