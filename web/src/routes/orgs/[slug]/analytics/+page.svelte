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

	const COLORS = ['#dc2626', '#3b82f6', '#10b981', '#f59e0b', '#8b5cf6', '#ec4899', '#06b6d4', '#84cc16'];

	interface TimeSeriesPoint {
		date: string;
		input: number;
		output: number;
	}
	interface RepoTokens {
		repo: string;
		tokens: number;
	}
	interface ModelCount {
		model: string;
		count: number;
	}
	interface RecentCommit {
		commit_sha: string;
		author: string;
		session_count: number;
		total_tokens: number;
		created_at: string;
	}
	interface SessionTimePoint {
		date: string;
		count: number;
	}
	interface HourlyActivity {
		hour: number;
		count: number;
	}
	interface OverviewResponse {
		total_commits: number;
		total_sessions: number;
		total_tokens: number;
		total_input_tokens: number;
		total_output_tokens: number;
		active_authors: number;
		estimated_cost_usd: number;
		ai_percentage: number | null;
		total_duration_ms: number;
		avg_session_duration_ms: number | null;
		total_tool_calls: number;
		total_compactions: number;
		total_compaction_tokens_saved: number;
		total_cache_read_tokens: number;
		total_cache_write_tokens: number;
		cache_savings_usd: number;
		tokens_over_time: TimeSeriesPoint[];
		sessions_over_time: SessionTimePoint[];
		hourly_activity: HourlyActivity[];
		top_repos: RepoTokens[];
		model_distribution: ModelCount[];
		recent_commits: RecentCommit[];
	}

	let data: OverviewResponse | null = $state(null);
	let loading = $state(true);
	let error = $state('');

	const slug = $derived($page.params.slug);

	async function fetchData(search: string) {
		loading = true;
		error = '';
		try {
			data = await api.get<OverviewResponse>(`/api/v1/orgs/${slug}/analytics/overview` + (search ? '?' + search : ''));
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load analytics';
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

	function fmtCost(n: number): string {
		return `$${n.toFixed(2)}`;
	}

	function fmtDate(iso: string): string {
		return new Date(iso).toLocaleDateString();
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

	function tokensChartData(d: OverviewResponse) {
		return {
			labels: d.tokens_over_time.map((p) => p.date),
			datasets: [
				{
					label: 'Input Tokens',
					data: d.tokens_over_time.map((p) => p.input),
					borderColor: '#3b82f6',
					backgroundColor: 'rgba(59,130,246,0.1)',
					fill: true,
					tension: 0.3
				},
				{
					label: 'Output Tokens',
					data: d.tokens_over_time.map((p) => p.output),
					borderColor: '#10b981',
					backgroundColor: 'rgba(16,185,129,0.1)',
					fill: true,
					tension: 0.3
				}
			]
		};
	}

	function reposChartData(d: OverviewResponse) {
		return {
			labels: d.top_repos.map((r) => r.repo),
			datasets: [
				{
					label: 'Tokens',
					data: d.top_repos.map((r) => r.tokens),
					backgroundColor: COLORS.slice(0, d.top_repos.length)
				}
			]
		};
	}

	function modelChartData(d: OverviewResponse) {
		return {
			labels: d.model_distribution.map((m) => m.model),
			datasets: [
				{
					data: d.model_distribution.map((m) => m.count),
					backgroundColor: COLORS.slice(0, d.model_distribution.length)
				}
			]
		};
	}

	function hourlyActivityChartData(d: OverviewResponse) {
		const hours = Array.from({ length: 24 }, (_, i) => i);
		const counts = hours.map((h) => d.hourly_activity.find((a) => a.hour === h)?.count ?? 0);
		return {
			labels: hours.map((h) => String(h)),
			datasets: [
				{
					label: 'Sessions',
					data: counts,
					backgroundColor: '#8b5cf6'
				}
			]
		};
	}

	function sessionsOverTimeChartData(d: OverviewResponse) {
		return {
			labels: d.sessions_over_time.map((p) => p.date),
			datasets: [
				{
					label: 'Sessions',
					data: d.sessions_over_time.map((p) => p.count),
					borderColor: '#f59e0b',
					backgroundColor: 'rgba(245,158,11,0.1)',
					fill: true,
					tension: 0.3
				}
			]
		};
	}
</script>

<svelte:head>
	<title>Analytics - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	<h1 class="text-xl font-semibold">Analytics Overview</h1>

	{#if loading}
		<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm">
			<span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"></span>
			Loading...
		</div>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if data}
		<div class="border-border overflow-hidden rounded-lg border">
			<div class="grid grid-cols-2 gap-px md:grid-cols-3 lg:grid-cols-6">
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Total Commits</div>
					<div class="mt-1 text-lg font-semibold">{fmtNum(data.total_commits)}</div>
				</div>
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Sessions</div>
					<div class="mt-1 text-lg font-semibold">{fmtNum(data.total_sessions)}</div>
				</div>
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Total Tokens</div>
					<div class="mt-1 text-lg font-semibold">{fmtNum(data.total_tokens)}</div>
				</div>
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Active Authors</div>
					<div class="mt-1 text-lg font-semibold">{data.active_authors}</div>
				</div>
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">AI %</div>
					<div class="mt-1 text-lg font-semibold">
						{data.ai_percentage != null ? `${data.ai_percentage.toFixed(1)}%` : 'N/A'}
					</div>
				</div>
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Estimated Cost</div>
					<div class="mt-1 text-lg font-semibold">{fmtCost(data.estimated_cost_usd)}</div>
				</div>
			</div>
		</div>

		<div class="border-border overflow-hidden rounded-lg border">
			<div class="grid grid-cols-2 gap-px md:grid-cols-3">
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Avg Session Duration</div>
					<div class="mt-1 text-lg font-semibold">{fmtDuration(data.avg_session_duration_ms)}</div>
				</div>
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Total Tool Calls</div>
					<div class="mt-1 text-lg font-semibold">{fmtNum(data.total_tool_calls)}</div>
				</div>
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Cache Savings</div>
					<div class="mt-1 text-lg font-semibold">{fmtCost(data.cache_savings_usd)}</div>
				</div>
			</div>
		</div>

		<div class="grid gap-6 lg:grid-cols-2">
			<div class="border-border rounded-lg border p-3">
				<h4 class="mb-2 text-sm font-semibold">
					<a href="/orgs/{slug}/analytics/tokens" class="hover:underline">Tokens Over Time</a>
				</h4>
				{#if data.tokens_over_time.length > 0}
					<Chart
						type="line"
						data={tokensChartData(data)}
						options={{ responsive: true, plugins: { legend: { position: 'top' } } }}
					/>
				{:else}
					<p class="text-muted-foreground text-sm">No data</p>
				{/if}
			</div>

			<div class="border-border rounded-lg border p-3">
				<h4 class="mb-2 text-sm font-semibold">
					<a href="/orgs/{slug}/analytics/tokens" class="hover:underline">Top Repos by Tokens</a>
				</h4>
				{#if data.top_repos.length > 0}
					<Chart
						type="bar"
						data={reposChartData(data)}
						options={{ responsive: true, indexAxis: 'y', plugins: { legend: { display: false } } }}
					/>
				{:else}
					<p class="text-muted-foreground text-sm">No data</p>
				{/if}
			</div>
		</div>

		<div class="grid gap-6 lg:grid-cols-2">
			<div class="border-border rounded-lg border p-3">
				<h4 class="mb-2 text-sm font-semibold">
					<a href="/orgs/{slug}/analytics/sessions" class="hover:underline">Hourly Activity</a>
				</h4>
				{#if data.hourly_activity.length > 0}
					<Chart
						type="bar"
						data={hourlyActivityChartData(data)}
						options={{
							responsive: true,
							plugins: { legend: { display: false } },
							scales: { x: { title: { display: true, text: 'Hour of Day' } }, y: { title: { display: true, text: 'Sessions' } } }
						}}
					/>
				{:else}
					<p class="text-muted-foreground text-sm">No data</p>
				{/if}
			</div>

			<div class="border-border rounded-lg border p-3">
				<h4 class="mb-2 text-sm font-semibold">
					<a href="/orgs/{slug}/analytics/sessions" class="hover:underline">Sessions Over Time</a>
				</h4>
				{#if data.sessions_over_time.length > 0}
					<Chart
						type="line"
						data={sessionsOverTimeChartData(data)}
						options={{ responsive: true, plugins: { legend: { position: 'top' } } }}
					/>
				{:else}
					<p class="text-muted-foreground text-sm">No data</p>
				{/if}
			</div>
		</div>

		<div class="grid gap-6 lg:grid-cols-2">
			<div class="border-border rounded-lg border p-3">
				<h4 class="mb-2 text-sm font-semibold">
					<a href="/orgs/{slug}/analytics/models" class="hover:underline">Model Distribution</a>
				</h4>
				<div class="flex justify-center">
					{#if data.model_distribution.length > 0}
						<div class="max-w-[300px]">
							<Chart
								type="doughnut"
								data={modelChartData(data)}
								options={{ responsive: true, plugins: { legend: { position: 'bottom' } } }}
							/>
						</div>
					{:else}
						<p class="text-muted-foreground text-sm">No data</p>
					{/if}
				</div>
			</div>

			<div class="border-border rounded-lg border p-3">
				<h4 class="mb-2 text-sm font-semibold">Recent Commits</h4>
				{#if data.recent_commits.length > 0}
					<Table.Root class="text-xs">
						<Table.Header>
							<Table.Row>
								<Table.Head>Commit</Table.Head>
								<Table.Head>Author</Table.Head>
								<Table.Head>Sessions</Table.Head>
								<Table.Head>Tokens</Table.Head>
								<Table.Head>Date</Table.Head>
							</Table.Row>
						</Table.Header>
						<Table.Body>
							{#each data.recent_commits as commit}
								<Table.Row class="hover:bg-muted/40 transition-colors">
									<Table.Cell class="font-mono">{commit.commit_sha.slice(0, 8)}</Table.Cell>
									<Table.Cell>{commit.author}</Table.Cell>
									<Table.Cell>{commit.session_count}</Table.Cell>
									<Table.Cell class="font-mono">{fmtNum(commit.total_tokens)}</Table.Cell>
									<Table.Cell>{fmtDate(commit.created_at)}</Table.Cell>
								</Table.Row>
							{/each}
						</Table.Body>
					</Table.Root>
				{:else}
					<p class="text-muted-foreground text-sm">No commits</p>
				{/if}
			</div>
		</div>
	{/if}
</div>
