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

	const COLORS = ['#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899', '#06b6d4', '#84cc16'];

	interface TokenTimePoint {
		date: string;
		input: number;
		output: number;
	}
	interface RepoTokenDetail {
		repo: string;
		total: number;
		input: number;
		output: number;
		sessions: number;
	}
	interface AuthorTokens {
		author: string;
		total: number;
	}
	interface TokensResponse {
		time_series: TokenTimePoint[];
		by_repo: RepoTokenDetail[];
		by_author: AuthorTokens[];
		cache_read_tokens: number;
		cache_write_tokens: number;
		cache_savings_usd: number;
	}

	let data: TokensResponse | null = $state(null);
	let loading = $state(true);
	let error = $state('');
	let sortCol = $state<'total' | 'input' | 'output' | 'sessions'>('total');
	let sortDir = $state<'asc' | 'desc'>('desc');

	const slug = $derived($page.params.slug);

	async function fetchData(search: string) {
		loading = true;
		error = '';
		try {
			data = await api.get<TokensResponse>(`/api/v1/orgs/${slug}/analytics/tokens` + (search ? '?' + search : ''));
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

	function sortBy(col: 'total' | 'input' | 'output' | 'sessions') {
		if (sortCol === col) {
			sortDir = sortDir === 'asc' ? 'desc' : 'asc';
		} else {
			sortCol = col;
			sortDir = 'desc';
		}
	}

	function sortedRepos(repos: RepoTokenDetail[]): RepoTokenDetail[] {
		return [...repos].sort((a, b) => {
			const diff = a[sortCol] - b[sortCol];
			return sortDir === 'asc' ? diff : -diff;
		});
	}

	function timeChartData(d: TokensResponse) {
		return {
			labels: d.time_series.map((p) => p.date),
			datasets: [
				{
					label: 'Input Tokens',
					data: d.time_series.map((p) => p.input),
					borderColor: '#3b82f6',
					backgroundColor: 'rgba(59,130,246,0.1)',
					fill: true,
					tension: 0.3
				},
				{
					label: 'Output Tokens',
					data: d.time_series.map((p) => p.output),
					borderColor: '#10b981',
					backgroundColor: 'rgba(16,185,129,0.1)',
					fill: true,
					tension: 0.3
				}
			]
		};
	}

	function authorChartData(d: TokensResponse) {
		return {
			labels: d.by_author.map((a) => a.author),
			datasets: [
				{
					label: 'Total Tokens',
					data: d.by_author.map((a) => a.total),
					backgroundColor: COLORS.slice(0, d.by_author.length)
				}
			]
		};
	}
</script>

<svelte:head>
	<title>Token Analytics - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	<h1 class="text-2xl font-bold">Token Analytics</h1>

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
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Cache Read Tokens</div>
					<div class="mt-1 text-lg font-semibold">{fmtNum(data.cache_read_tokens)}</div>
				</div>
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Cache Write Tokens</div>
					<div class="mt-1 text-lg font-semibold">{fmtNum(data.cache_write_tokens)}</div>
				</div>
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Cache Savings</div>
					<div class="mt-1 text-lg font-semibold">${data.cache_savings_usd.toFixed(2)}</div>
				</div>
			</div>
		</div>

		<div class="border-border rounded-lg border p-3">
			<h4 class="mb-2 text-sm font-semibold">Tokens Over Time</h4>
			{#if data.time_series.length > 0}
				<Chart
					type="line"
					data={timeChartData(data)}
					options={{ responsive: true, plugins: { legend: { position: 'top' } } }}
				/>
			{:else}
				<p class="text-muted-foreground text-sm">No data</p>
			{/if}
		</div>

		<div class="border-border overflow-hidden rounded-lg border">
			<h2 class="text-sm font-semibold uppercase tracking-wide text-muted-foreground p-3">By Repository</h2>
			{#if data.by_repo.length > 0}
				<Table.Root class="text-xs">
					<Table.Header>
						<Table.Row>
							<Table.Head>Repo</Table.Head>
							<Table.Head>
								<button class="hover:underline" onclick={() => sortBy('total')}>
									Total Tokens {sortCol === 'total' ? (sortDir === 'asc' ? '↑' : '↓') : ''}
								</button>
							</Table.Head>
							<Table.Head>
								<button class="hover:underline" onclick={() => sortBy('input')}>
									Input {sortCol === 'input' ? (sortDir === 'asc' ? '↑' : '↓') : ''}
								</button>
							</Table.Head>
							<Table.Head>
								<button class="hover:underline" onclick={() => sortBy('output')}>
									Output {sortCol === 'output' ? (sortDir === 'asc' ? '↑' : '↓') : ''}
								</button>
							</Table.Head>
							<Table.Head>
								<button class="hover:underline" onclick={() => sortBy('sessions')}>
									Sessions {sortCol === 'sessions' ? (sortDir === 'asc' ? '↑' : '↓') : ''}
								</button>
							</Table.Head>
							<Table.Head>Avg/Session</Table.Head>
						</Table.Row>
					</Table.Header>
					<Table.Body>
						{#each sortedRepos(data.by_repo) as repo}
							<Table.Row class="hover:bg-muted/40 transition-colors">
								<Table.Cell>{repo.repo}</Table.Cell>
								<Table.Cell class="font-mono">{fmtNum(repo.total)}</Table.Cell>
								<Table.Cell class="font-mono">{fmtNum(repo.input)}</Table.Cell>
								<Table.Cell class="font-mono">{fmtNum(repo.output)}</Table.Cell>
								<Table.Cell>{repo.sessions}</Table.Cell>
								<Table.Cell class="font-mono">
									{repo.sessions > 0 ? fmtNum(Math.round(repo.total / repo.sessions)) : '-'}
								</Table.Cell>
							</Table.Row>
						{/each}
					</Table.Body>
				</Table.Root>
			{:else}
				<p class="text-muted-foreground text-sm p-3">No data</p>
			{/if}
		</div>

		<div class="border-border rounded-lg border p-3">
			<h4 class="mb-2 text-sm font-semibold">By Author</h4>
			{#if data.by_author.length > 0}
				<Chart
					type="bar"
					data={authorChartData(data)}
					options={{ responsive: true, plugins: { legend: { display: false } } }}
				/>
			{:else}
				<p class="text-muted-foreground text-sm">No data</p>
			{/if}
		</div>
	{/if}
</div>
