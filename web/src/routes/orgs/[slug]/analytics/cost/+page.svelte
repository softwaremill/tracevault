<script lang="ts">
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import { features } from '$lib/stores/features';
	import EnterpriseUpgrade from '$lib/components/enterprise-upgrade.svelte';
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

	interface CostTimePoint {
		date: string;
		cost: number;
	}
	interface ModelCost {
		model: string;
		cost: number;
		tokens: number;
		sessions: number;
	}
	interface RepoCost {
		repo: string;
		cost: number;
	}
	interface AuthorCost {
		author: string;
		cost: number;
	}
	interface CostResponse {
		total_cost: number;
		avg_cost_per_session: number;
		cache_savings_usd: number;
		cost_over_time: CostTimePoint[];
		cost_by_model: ModelCost[];
		cost_by_repo: RepoCost[];
		cost_by_author: AuthorCost[];
	}

	let data: CostResponse | null = $state(null);
	let loading = $state(true);
	let error = $state('');

	const slug = $derived($page.params.slug);

	async function fetchData(search: string) {
		loading = true;
		error = '';
		try {
			data = await api.get<CostResponse>(`/api/v1/orgs/${slug}/analytics/cost` + (search ? '?' + search : ''));
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

	function fmtCost(n: number): string {
		return `$${n.toFixed(2)}`;
	}

	function costOverTimeChartData(d: CostResponse) {
		return {
			labels: d.cost_over_time.map((p) => p.date),
			datasets: [
				{
					label: 'Cost ($)',
					data: d.cost_over_time.map((p) => p.cost),
					borderColor: '#3b82f6',
					backgroundColor: 'rgba(59,130,246,0.1)',
					fill: true,
					tension: 0.3
				}
			]
		};
	}

	function costByModelChartData(d: CostResponse) {
		return {
			labels: d.cost_by_model.map((m) => m.model),
			datasets: [
				{
					data: d.cost_by_model.map((m) => m.cost),
					backgroundColor: COLORS.slice(0, d.cost_by_model.length)
				}
			]
		};
	}

	function costByRepoChartData(d: CostResponse) {
		return {
			labels: d.cost_by_repo.map((r) => r.repo),
			datasets: [
				{
					label: 'Cost ($)',
					data: d.cost_by_repo.map((r) => r.cost),
					backgroundColor: COLORS.slice(0, d.cost_by_repo.length)
				}
			]
		};
	}

	function costByAuthorChartData(d: CostResponse) {
		return {
			labels: d.cost_by_author.map((a) => a.author),
			datasets: [
				{
					label: 'Cost ($)',
					data: d.cost_by_author.map((a) => a.cost),
					backgroundColor: COLORS.slice(0, d.cost_by_author.length)
				}
			]
		};
	}
</script>

<svelte:head>
	<title>Cost Analytics - TraceVault</title>
</svelte:head>

{#if !$features.loaded}
	<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm">
		<span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"></span>
		Loading...
	</div>
{:else if $features.advanced_analytics}
<div class="space-y-6">
	<h1 class="text-2xl font-bold">Cost Analytics</h1>

	{#if loading}
		<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm">
			<span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"></span>
			Loading...
		</div>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if data}
		<div class="border-border overflow-hidden rounded-lg border">
			<div class="grid grid-cols-2 gap-px md:grid-cols-3">
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Total Cost</div>
					<div class="mt-1 text-lg font-semibold">{fmtCost(data.total_cost)}</div>
				</div>
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Cache Savings</div>
					<div class="mt-1 text-lg font-semibold">{fmtCost(data.cache_savings_usd)}</div>
				</div>
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Avg Cost/Session</div>
					<div class="mt-1 text-lg font-semibold">{fmtCost(data.avg_cost_per_session)}</div>
				</div>
			</div>
		</div>

		<div class="border-border rounded-lg border p-3">
			<h4 class="mb-2 text-sm font-semibold">Cost Over Time</h4>
			{#if data.cost_over_time.length > 0}
				<Chart
					type="line"
					data={costOverTimeChartData(data)}
					options={{ responsive: true, plugins: { legend: { position: 'top' } } }}
				/>
			{:else}
				<p class="text-muted-foreground text-sm">No data</p>
			{/if}
		</div>

		<div class="grid gap-6 lg:grid-cols-2">
			<div class="border-border rounded-lg border p-3">
				<h4 class="mb-2 text-sm font-semibold">Cost by Model</h4>
				{#if data.cost_by_model.length > 0}
					<div class="flex justify-center">
						<div class="max-w-[300px]">
							<Chart
								type="doughnut"
								data={costByModelChartData(data)}
								options={{ responsive: true, plugins: { legend: { position: 'bottom' } } }}
							/>
						</div>
					</div>
				{:else}
					<p class="text-muted-foreground text-sm">No data</p>
				{/if}
			</div>

			<div class="border-border rounded-lg border p-3">
				<h4 class="mb-2 text-sm font-semibold">Cost by Repository</h4>
				{#if data.cost_by_repo.length > 0}
					<Chart
						type="bar"
						data={costByRepoChartData(data)}
						options={{ responsive: true, indexAxis: 'y', plugins: { legend: { display: false } } }}
					/>
				{:else}
					<p class="text-muted-foreground text-sm">No data</p>
				{/if}
			</div>
		</div>

		<div class="border-border rounded-lg border p-3">
			<h4 class="mb-2 text-sm font-semibold">Cost by Author</h4>
			{#if data.cost_by_author.length > 0}
				<Chart
					type="bar"
					data={costByAuthorChartData(data)}
					options={{ responsive: true, indexAxis: 'y', plugins: { legend: { display: false } } }}
				/>
			{:else}
				<p class="text-muted-foreground text-sm">No data</p>
			{/if}
		</div>
	{/if}
</div>
{:else}
	<div class="border-border overflow-hidden rounded-lg border">
		<EnterpriseUpgrade feature="advanced_analytics" />
	</div>
{/if}
