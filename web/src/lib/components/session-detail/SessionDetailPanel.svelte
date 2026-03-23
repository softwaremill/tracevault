<script lang="ts">
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import SessionSummaryStats from './SessionSummaryStats.svelte';
	import SessionCharts from './SessionCharts.svelte';
	import SessionTranscript from './SessionTranscript.svelte';

	interface Props {
		sessionId: string;
	}

	let { sessionId }: Props = $props();

	const slug = $derived($page.params.slug);

	let data: any = $state(null);
	let loading = $state(true);
	let error = $state('');
	let showDetail = $state(false);

	async function fetchDetail() {
		loading = true;
		error = '';
		try {
			data = await api.get(`/api/v1/orgs/${slug}/analytics/sessions/${sessionId}/detail`);
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load session detail';
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		fetchDetail();
	});
</script>

<div class="bg-muted/20 border-border border-t px-4 py-4">
	{#if loading}
		<div class="text-muted-foreground flex items-center justify-center gap-2 py-8 text-sm">
			<span
				class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
			></span>
			Loading session detail...
		</div>
	{:else if error}
		<div class="text-destructive py-4 text-center text-sm">{error}</div>
	{:else if data}
		<SessionSummaryStats
			totalTokens={data.total_tokens}
			estimatedCostUsd={data.estimated_cost_usd}
			outputTokens={data.output_tokens}
			apiCalls={data.api_calls}
			cacheSavings={data.cache_savings}
			compactions={data.compactions}
			costBreakdown={data.cost_breakdown}
		/>

		{#if !showDetail}
			<div class="mt-4 text-center">
				<button
					class="bg-muted hover:bg-muted/80 border-border rounded-md border px-5 py-2 text-sm transition-colors"
					style="color: #4f6ef7"
					onclick={() => (showDetail = true)}
				>
					Show detailed charts & transcript ▼
				</button>
			</div>
		{:else}
			<div class="mt-4 space-y-6">
				<SessionCharts perCall={data.per_call} tokenDistribution={data.token_distribution} />
				<SessionTranscript records={data.transcript_records} />
			</div>
		{/if}
	{/if}
</div>
