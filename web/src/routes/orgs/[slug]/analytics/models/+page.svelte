<script lang="ts">
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import * as Table from '$lib/components/ui/table/index.js';
	import Chart from '$lib/components/chart.svelte';
	import {
		Chart as ChartJS,
		CategoryScale,
		LinearScale,
		PointElement,
		LineElement,
		BarElement,
		ArcElement,
		Title,
		Tooltip,
		Legend,
		Filler
	} from 'chart.js';

	ChartJS.register(
		CategoryScale,
		LinearScale,
		PointElement,
		LineElement,
		BarElement,
		ArcElement,
		Title,
		Tooltip,
		Legend,
		Filler
	);

	const COLORS = ['#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899', '#06b6d4', '#84cc16'];

	interface ModelDistribution {
		model: string;
		session_count: number;
		total_tokens: number;
	}
	interface ModelTrend {
		date: string;
		model: string;
		count: number;
	}
	interface AuthorModel {
		author: string;
		model: string;
		sessions: number;
		tokens: number;
	}
	interface ModelComparison {
		model: string;
		avg_tokens: number;
		avg_cost: number;
		cache_read_tokens: number;
		cache_write_tokens: number;
		avg_duration_ms: number | null;
	}
	interface ModelsResponse {
		distribution: ModelDistribution[];
		trends: ModelTrend[];
		author_model_matrix: AuthorModel[];
		comparison: ModelComparison[];
	}

	let data: ModelsResponse | null = $state(null);
	let loading = $state(true);
	let error = $state('');

	const slug = $derived($page.params.slug);

	async function fetchData(search: string) {
		loading = true;
		error = '';
		try {
			data = await api.get<ModelsResponse>(`/api/v1/orgs/${slug}/analytics/models` + (search ? '?' + search : ''));
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load';
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		const search = $page.url.search.replace(/^\?/, '');
		fetchData(search);
	});

	function fmtNum(n: number): string {
		if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
		if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
		return String(n);
	}

	function fmtDuration(ms: number | null): string {
		if (ms == null) return '-';
		const totalSeconds = Math.floor(ms / 1000);
		const hours = Math.floor(totalSeconds / 3600);
		const minutes = Math.floor((totalSeconds % 3600) / 60);
		const seconds = totalSeconds % 60;
		if (hours >= 1) return `${hours}h ${minutes}m`;
		if (minutes >= 1) return `${minutes}m ${seconds}s`;
		return `${seconds}s`;
	}

	function distributionChartData(d: ModelsResponse) {
		return {
			labels: d.distribution.map((m) => m.model),
			datasets: [
				{
					data: d.distribution.map((m) => m.session_count),
					backgroundColor: COLORS.slice(0, d.distribution.length)
				}
			]
		};
	}

	function trendsChartData(d: ModelsResponse) {
		const models = [...new Set(d.trends.map((t) => t.model))];
		const dates = [...new Set(d.trends.map((t) => t.date))].sort();
		return {
			labels: dates,
			datasets: models.map((model, i) => ({
				label: model,
				data: dates.map((date) => d.trends.find((t) => t.date === date && t.model === model)?.count ?? 0),
				borderColor: COLORS[i % COLORS.length],
				backgroundColor: COLORS[i % COLORS.length] + '33',
				fill: true,
				tension: 0.3
			}))
		};
	}

	function comparisonChartData(d: ModelsResponse) {
		return {
			labels: d.comparison.map((m) => m.model),
			datasets: [
				{
					label: 'Avg Tokens',
					data: d.comparison.map((m) => m.avg_tokens),
					backgroundColor: '#3b82f6'
				}
			]
		};
	}
</script>

<svelte:head>
	<title>Model Analytics - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	<h1 class="text-2xl font-bold">Model Analytics</h1>

	{#if loading}
		<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm">
			<span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"></span>
			Loading...
		</div>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if data}
		<div class="grid gap-6 lg:grid-cols-2">
			<div class="border-border rounded-lg border p-3">
				<h4 class="mb-2 text-sm font-semibold">Model Distribution</h4>
				{#if data.distribution.length > 0}
					<div class="flex justify-center">
						<div class="max-w-[300px]">
							<Chart
								type="doughnut"
								data={distributionChartData(data)}
								options={{ responsive: true, plugins: { legend: { position: 'bottom' } } }}
							/>
						</div>
					</div>
				{:else}
					<p class="text-muted-foreground text-sm">No data</p>
				{/if}
			</div>

			<div class="border-border rounded-lg border p-3">
				<h4 class="mb-2 text-sm font-semibold">Model Trends</h4>
				{#if data.trends.length > 0}
					<Chart
						type="line"
						data={trendsChartData(data)}
						options={{
							responsive: true,
							scales: { y: { stacked: true } },
							plugins: { legend: { position: 'top' } }
						}}
					/>
				{:else}
					<p class="text-muted-foreground text-sm">No data</p>
				{/if}
			</div>
		</div>

		<div class="border-border overflow-hidden rounded-lg border">
			<h2 class="text-sm font-semibold uppercase tracking-wide text-muted-foreground p-3">Author x Model Matrix</h2>
			{#if data.author_model_matrix.length > 0}
				<Table.Root class="text-xs">
					<Table.Header>
						<Table.Row>
							<Table.Head>Author</Table.Head>
							<Table.Head>Model</Table.Head>
							<Table.Head>Sessions</Table.Head>
							<Table.Head>Tokens</Table.Head>
						</Table.Row>
					</Table.Header>
					<Table.Body>
						{#each data.author_model_matrix as row}
							<Table.Row class="hover:bg-muted/40 transition-colors">
								<Table.Cell>{row.author}</Table.Cell>
								<Table.Cell>
									<span class="rounded-full px-2 py-0.5 text-[10px]" style="background: rgba(167,139,250,0.12); color: #a78bfa; border: 1px solid rgba(167,139,250,0.25)">{row.model}</span>
								</Table.Cell>
								<Table.Cell>{row.sessions}</Table.Cell>
								<Table.Cell class="font-mono">{fmtNum(row.tokens)}</Table.Cell>
							</Table.Row>
						{/each}
					</Table.Body>
				</Table.Root>
			{:else}
				<p class="text-muted-foreground text-sm p-3">No data</p>
			{/if}
		</div>

		<div class="border-border overflow-hidden rounded-lg border">
			<h2 class="text-sm font-semibold uppercase tracking-wide text-muted-foreground p-3">Model Comparison</h2>
			{#if data.comparison.length > 0}
				<div class="p-3 pt-0">
					<Chart
						type="bar"
						data={comparisonChartData(data)}
						options={{ responsive: true, plugins: { legend: { display: false } } }}
					/>
				</div>
				<Table.Root class="text-xs">
					<Table.Header>
						<Table.Row>
							<Table.Head>Model</Table.Head>
							<Table.Head>Avg Tokens</Table.Head>
							<Table.Head>Avg Cost</Table.Head>
							<Table.Head>Cache Read</Table.Head>
							<Table.Head>Cache Write</Table.Head>
							<Table.Head>Avg Duration</Table.Head>
						</Table.Row>
					</Table.Header>
					<Table.Body>
						{#each data.comparison as row}
							<Table.Row class="hover:bg-muted/40 transition-colors">
								<Table.Cell>
									<span class="rounded-full px-2 py-0.5 text-[10px]" style="background: rgba(167,139,250,0.12); color: #a78bfa; border: 1px solid rgba(167,139,250,0.25)">{row.model}</span>
								</Table.Cell>
								<Table.Cell class="font-mono">{fmtNum(row.avg_tokens)}</Table.Cell>
								<Table.Cell class="font-mono">${row.avg_cost.toFixed(4)}</Table.Cell>
								<Table.Cell class="font-mono">{fmtNum(row.cache_read_tokens)}</Table.Cell>
								<Table.Cell class="font-mono">{fmtNum(row.cache_write_tokens)}</Table.Cell>
								<Table.Cell class="font-mono">{fmtDuration(row.avg_duration_ms)}</Table.Cell>
							</Table.Row>
						{/each}
					</Table.Body>
				</Table.Root>
			{:else}
				<p class="text-muted-foreground text-sm p-3">No data</p>
			{/if}
		</div>
	{/if}
</div>
