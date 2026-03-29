<script lang="ts">
	import { page } from '$app/stores';
	import { useFetch } from '$lib/hooks/use-fetch.svelte';
	import { fmtNum } from '$lib/utils/format';
	import Chart from '$lib/components/chart.svelte';
	import HelpTip from '$lib/components/HelpTip.svelte';
	import LoadingState from '$lib/components/LoadingState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
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

	const slug = $derived($page.params.slug);
	const search = $derived($page.url.search.replace(/^\?/, ''));

	const attributionQuery = useFetch<AttributionResponse>(
		() => `/api/v1/orgs/${slug}/analytics/attribution` + (search ? '?' + search : '')
	);

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

	{#if attributionQuery.loading}
		<LoadingState />
	{:else if attributionQuery.error}
		<ErrorState message={attributionQuery.error} onRetry={attributionQuery.refetch} />
	{:else if attributionQuery.data}
		{@const data = attributionQuery.data}
		<div class="border-border overflow-hidden rounded-lg border">
			<div class="grid grid-cols-3 gap-px">
				<div class="bg-background p-3 text-center">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">AI Lines<HelpTip text="Total lines of code attributed to AI across all commits." /></div>
					<div class="mt-1 text-lg font-semibold" style="color: {AI_COLOR}">{fmtNum(data.totals.ai_lines)}</div>
				</div>
				<div class="bg-background p-3 text-center">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Human Lines<HelpTip text="Total lines of code written by humans." /></div>
					<div class="mt-1 text-lg font-semibold" style="color: {HUMAN_COLOR}">{fmtNum(data.totals.human_lines)}</div>
				</div>
				<div class="bg-background p-3 text-center">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Overall AI %<HelpTip text="Percentage of all code lines attributed to AI assistance." /></div>
					<div class="mt-1 text-lg font-semibold">{data.totals.ai_pct.toFixed(1)}%</div>
				</div>
			</div>
		</div>

		<div class="border-border rounded-lg border p-3">
			<h4 class="mb-2 text-sm font-semibold">AI vs Human Trend<HelpTip text="How the ratio of AI-generated vs human-written code has changed over time." /></h4>
			{#if data.trend.length > 0}
				<div style="height: 200px">
					<Chart
						type="line"
						data={trendChartData(data)}
						options={{
							responsive: true,
							maintainAspectRatio: false,
							scales: { y: { stacked: true, max: 100 } },
							plugins: { legend: { position: 'bottom', labels: { boxWidth: 12, font: { size: 11 } } } }
						}}
					/>
				</div>
			{:else}
				<p class="text-muted-foreground text-sm">No data</p>
			{/if}
		</div>

		<div class="grid gap-4 lg:grid-cols-2">
			<div class="border-border rounded-lg border p-3">
				<h4 class="mb-2 text-sm font-semibold">By Repository<HelpTip text="AI vs human code ratio per repository." /></h4>
				{#if data.by_repo.length > 0}
					<div style="height: 200px">
						<Chart
							type="bar"
							data={repoChartData(data)}
							options={{
								responsive: true,
								maintainAspectRatio: false,
								indexAxis: 'y',
								scales: { x: { stacked: true, max: 100 } },
								plugins: { legend: { position: 'bottom', labels: { boxWidth: 12, font: { size: 11 } } } }
							}}
						/>
					</div>
				{:else}
					<p class="text-muted-foreground text-sm">No data</p>
				{/if}
			</div>

			<div class="border-border rounded-lg border p-3">
				<h4 class="mb-2 text-sm font-semibold">By Author<HelpTip text="AI vs human code ratio per developer." /></h4>
				{#if data.by_author.length > 0}
					<div style="height: 200px">
						<Chart
							type="bar"
							data={authorChartData(data)}
							options={{
								responsive: true,
								maintainAspectRatio: false,
								indexAxis: 'y',
								scales: { x: { stacked: true, max: 100 } },
								plugins: { legend: { position: 'bottom', labels: { boxWidth: 12, font: { size: 11 } } } }
							}}
						/>
					</div>
				{:else}
					<p class="text-muted-foreground text-sm">No data</p>
				{/if}
			</div>
		</div>
	{/if}
</div>
