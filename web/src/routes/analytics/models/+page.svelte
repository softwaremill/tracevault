<script lang="ts">
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Table from '$lib/components/ui/table/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
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

	async function fetchData(search: string) {
		loading = true;
		error = '';
		try {
			data = await api.get<ModelsResponse>('/api/v1/analytics/models' + (search ? '?' + search : ''));
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
		<p class="text-muted-foreground">Loading...</p>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if data}
		<div class="grid gap-6 lg:grid-cols-2">
			<Card.Root>
				<Card.Header>
					<Card.Title>Model Distribution</Card.Title>
				</Card.Header>
				<Card.Content class="flex justify-center">
					{#if data.distribution.length > 0}
						<div class="max-w-[300px]">
							<Chart
								type="doughnut"
								data={distributionChartData(data)}
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
					<Card.Title>Model Trends</Card.Title>
				</Card.Header>
				<Card.Content>
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
				</Card.Content>
			</Card.Root>
		</div>

		<Card.Root>
			<Card.Header>
				<Card.Title>Author x Model Matrix</Card.Title>
			</Card.Header>
			<Card.Content>
				{#if data.author_model_matrix.length > 0}
					<Table.Root>
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
								<Table.Row>
									<Table.Cell>{row.author}</Table.Cell>
									<Table.Cell><Badge variant="outline">{row.model}</Badge></Table.Cell>
									<Table.Cell>{row.sessions}</Table.Cell>
									<Table.Cell class="font-mono text-sm">{fmtNum(row.tokens)}</Table.Cell>
								</Table.Row>
							{/each}
						</Table.Body>
					</Table.Root>
				{:else}
					<p class="text-muted-foreground text-sm">No data</p>
				{/if}
			</Card.Content>
		</Card.Root>

		<Card.Root>
			<Card.Header>
				<Card.Title>Model Comparison</Card.Title>
			</Card.Header>
			<Card.Content>
				{#if data.comparison.length > 0}
					<Chart
					type="bar"
						data={comparisonChartData(data)}
						options={{ responsive: true, plugins: { legend: { display: false } } }}
					/>
					<Table.Root class="mt-4">
						<Table.Header>
							<Table.Row>
								<Table.Head>Model</Table.Head>
								<Table.Head>Avg Tokens</Table.Head>
								<Table.Head>Avg Cost</Table.Head>
							</Table.Row>
						</Table.Header>
						<Table.Body>
							{#each data.comparison as row}
								<Table.Row>
									<Table.Cell><Badge variant="outline">{row.model}</Badge></Table.Cell>
									<Table.Cell class="font-mono text-sm">{fmtNum(row.avg_tokens)}</Table.Cell>
									<Table.Cell class="font-mono text-sm">${row.avg_cost.toFixed(4)}</Table.Cell>
								</Table.Row>
							{/each}
						</Table.Body>
					</Table.Root>
				{:else}
					<p class="text-muted-foreground text-sm">No data</p>
				{/if}
			</Card.Content>
		</Card.Root>
	{/if}
</div>
