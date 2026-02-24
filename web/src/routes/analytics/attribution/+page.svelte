<script lang="ts">
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import * as Card from '$lib/components/ui/card/index.js';
	import { Line, Bar } from 'svelte-chartjs';
	import {
		Chart as ChartJS,
		CategoryScale,
		LinearScale,
		PointElement,
		LineElement,
		BarElement,
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
		Title,
		Tooltip,
		Legend,
		Filler
	);

	interface AttributionTrend {
		date: string;
		ai_pct: number;
		human_pct: number;
	}
	interface RepoAttribution {
		repo: string;
		ai_pct: number;
		ai_lines: number;
		human_lines: number;
	}
	interface AuthorAttribution {
		author: string;
		ai_pct: number;
	}
	interface AttributionTotals {
		ai_lines: number;
		human_lines: number;
		ai_pct: number;
	}
	interface AttributionResponse {
		trend: AttributionTrend[];
		by_repo: RepoAttribution[];
		by_author: AuthorAttribution[];
		totals: AttributionTotals;
	}

	let data: AttributionResponse | null = $state(null);
	let loading = $state(true);
	let error = $state('');

	async function fetchData(search: string) {
		loading = true;
		error = '';
		try {
			data = await api.get<AttributionResponse>('/api/v1/analytics/attribution' + (search ? '?' + search : ''));
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

	const AI_COLOR = '#8b5cf6';
	const HUMAN_COLOR = '#10b981';

	function trendChartData(d: AttributionResponse) {
		return {
			labels: d.trend.map((t) => t.date),
			datasets: [
				{
					label: 'AI %',
					data: d.trend.map((t) => t.ai_pct),
					borderColor: AI_COLOR,
					backgroundColor: AI_COLOR + '33',
					fill: true,
					tension: 0.3
				},
				{
					label: 'Human %',
					data: d.trend.map((t) => t.human_pct),
					borderColor: HUMAN_COLOR,
					backgroundColor: HUMAN_COLOR + '33',
					fill: true,
					tension: 0.3
				}
			]
		};
	}

	function repoChartData(d: AttributionResponse) {
		return {
			labels: d.by_repo.map((r) => r.repo),
			datasets: [
				{
					label: 'AI %',
					data: d.by_repo.map((r) => r.ai_pct),
					backgroundColor: AI_COLOR
				},
				{
					label: 'Human %',
					data: d.by_repo.map((r) => 100 - r.ai_pct),
					backgroundColor: HUMAN_COLOR
				}
			]
		};
	}

	function authorChartData(d: AttributionResponse) {
		return {
			labels: d.by_author.map((a) => a.author),
			datasets: [
				{
					label: 'AI %',
					data: d.by_author.map((a) => a.ai_pct),
					backgroundColor: AI_COLOR
				},
				{
					label: 'Human %',
					data: d.by_author.map((a) => 100 - a.ai_pct),
					backgroundColor: HUMAN_COLOR
				}
			]
		};
	}
</script>

<svelte:head>
	<title>Attribution Analytics - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	<h1 class="text-2xl font-bold">AI Attribution Analytics</h1>

	{#if loading}
		<p class="text-muted-foreground">Loading...</p>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if data}
		<Card.Root>
			<Card.Header>
				<Card.Title>Summary</Card.Title>
			</Card.Header>
			<Card.Content>
				<div class="grid grid-cols-3 gap-4 text-center">
					<div>
						<p class="text-muted-foreground text-sm">AI Lines</p>
						<p class="text-2xl font-bold" style="color: {AI_COLOR}">{fmtNum(data.totals.ai_lines)}</p>
					</div>
					<div>
						<p class="text-muted-foreground text-sm">Human Lines</p>
						<p class="text-2xl font-bold" style="color: {HUMAN_COLOR}">{fmtNum(data.totals.human_lines)}</p>
					</div>
					<div>
						<p class="text-muted-foreground text-sm">Overall AI %</p>
						<p class="text-2xl font-bold">{data.totals.ai_pct.toFixed(1)}%</p>
					</div>
				</div>
			</Card.Content>
		</Card.Root>

		<Card.Root>
			<Card.Header>
				<Card.Title>AI vs Human Trend</Card.Title>
			</Card.Header>
			<Card.Content>
				{#if data.trend.length > 0}
					<Line
						data={trendChartData(data)}
						options={{
							responsive: true,
							scales: { y: { stacked: true, max: 100 } },
							plugins: { legend: { position: 'top' } }
						}}
					/>
				{:else}
					<p class="text-muted-foreground text-sm">No data</p>
				{/if}
			</Card.Content>
		</Card.Root>

		<div class="grid gap-6 lg:grid-cols-2">
			<Card.Root>
				<Card.Header>
					<Card.Title>By Repository</Card.Title>
				</Card.Header>
				<Card.Content>
					{#if data.by_repo.length > 0}
						<Bar
							data={repoChartData(data)}
							options={{
								responsive: true,
								indexAxis: 'y',
								scales: { x: { stacked: true, max: 100 } },
								plugins: { legend: { position: 'top' } }
							}}
						/>
					{:else}
						<p class="text-muted-foreground text-sm">No data</p>
					{/if}
				</Card.Content>
			</Card.Root>

			<Card.Root>
				<Card.Header>
					<Card.Title>By Author</Card.Title>
				</Card.Header>
				<Card.Content>
					{#if data.by_author.length > 0}
						<Bar
							data={authorChartData(data)}
							options={{
								responsive: true,
								indexAxis: 'y',
								scales: { x: { stacked: true, max: 100 } },
								plugins: { legend: { position: 'top' } }
							}}
						/>
					{:else}
						<p class="text-muted-foreground text-sm">No data</p>
					{/if}
				</Card.Content>
			</Card.Root>
		</div>
	{/if}
</div>
