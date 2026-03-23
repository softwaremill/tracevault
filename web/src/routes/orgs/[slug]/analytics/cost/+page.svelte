<script lang="ts">
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import { features } from '$lib/stores/features';
	import EnterpriseUpgrade from '$lib/components/enterprise-upgrade.svelte';
	import * as Card from '$lib/components/ui/card/index.js';
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

{#if $features.advanced_analytics}
<div class="space-y-6">
	<h1 class="text-2xl font-bold">Cost Analytics</h1>

	{#if loading}
		<p class="text-muted-foreground">Loading...</p>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if data}
		<div class="grid grid-cols-1 gap-4 md:grid-cols-3">
			<Card.Root>
				<Card.Header class="pb-2">
					<Card.Description>Total Cost</Card.Description>
				</Card.Header>
				<Card.Content>
					<p class="text-2xl font-bold">{fmtCost(data.total_cost)}</p>
				</Card.Content>
			</Card.Root>
			<Card.Root>
				<Card.Header class="pb-2">
					<Card.Description>Cache Savings</Card.Description>
				</Card.Header>
				<Card.Content>
					<p class="text-2xl font-bold">{fmtCost(data.cache_savings_usd)}</p>
				</Card.Content>
			</Card.Root>
			<Card.Root>
				<Card.Header class="pb-2">
					<Card.Description>Avg Cost/Session</Card.Description>
				</Card.Header>
				<Card.Content>
					<p class="text-2xl font-bold">{fmtCost(data.avg_cost_per_session)}</p>
				</Card.Content>
			</Card.Root>
		</div>

		<Card.Root>
			<Card.Header>
				<Card.Title>Cost Over Time</Card.Title>
			</Card.Header>
			<Card.Content>
				{#if data.cost_over_time.length > 0}
					<Chart
						type="line"
						data={costOverTimeChartData(data)}
						options={{ responsive: true, plugins: { legend: { position: 'top' } } }}
					/>
				{:else}
					<p class="text-muted-foreground text-sm">No data</p>
				{/if}
			</Card.Content>
		</Card.Root>

		<div class="grid gap-6 lg:grid-cols-2">
			<Card.Root>
				<Card.Header>
					<Card.Title>Cost by Model</Card.Title>
				</Card.Header>
				<Card.Content class="flex justify-center">
					{#if data.cost_by_model.length > 0}
						<div class="max-w-[300px]">
							<Chart
								type="doughnut"
								data={costByModelChartData(data)}
								options={{ responsive: true, plugins: { legend: { position: 'bottom' } } }}
							/>
						</div>
					{:else}
						<p class="text-muted-foreground text-sm">No data</p>
					{/if}
				</Card.Content>
			</Card.Root>

			<Card.Root>
				<Card.Header>
					<Card.Title>Cost by Repository</Card.Title>
				</Card.Header>
				<Card.Content>
					{#if data.cost_by_repo.length > 0}
						<Chart
							type="bar"
							data={costByRepoChartData(data)}
							options={{ responsive: true, indexAxis: 'y', plugins: { legend: { display: false } } }}
						/>
					{:else}
						<p class="text-muted-foreground text-sm">No data</p>
					{/if}
				</Card.Content>
			</Card.Root>
		</div>

		<Card.Root>
			<Card.Header>
				<Card.Title>Cost by Author</Card.Title>
			</Card.Header>
			<Card.Content>
				{#if data.cost_by_author.length > 0}
					<Chart
						type="bar"
						data={costByAuthorChartData(data)}
						options={{ responsive: true, indexAxis: 'y', plugins: { legend: { display: false } } }}
					/>
				{:else}
					<p class="text-muted-foreground text-sm">No data</p>
				{/if}
			</Card.Content>
		</Card.Root>
	{/if}
</div>
{:else}
	<EnterpriseUpgrade feature="advanced_analytics" />
{/if}
