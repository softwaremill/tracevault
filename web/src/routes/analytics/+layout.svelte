<script lang="ts">
	import AppLayout from '$lib/components/app-layout.svelte';
	import AnalyticsFilters from '$lib/components/analytics-filters.svelte';
	let { children } = $props();
</script>

<AppLayout>
	<div class="space-y-4">
		<AnalyticsFilters />
		{@render children()}
	</div>
</AppLayout>
