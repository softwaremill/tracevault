<script lang="ts">
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import StatCard from '$lib/components/StatCard.svelte';
	import DataTable from '$lib/components/DataTable.svelte';
	import Chart from '$lib/components/chart.svelte';
	import BookOpenIcon from '@lucide/svelte/icons/book-open';
	import BookMarkedIcon from '@lucide/svelte/icons/book-marked';
	import PiggyBankIcon from '@lucide/svelte/icons/piggy-bank';
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

	const repoColumns = [
		{ key: 'repo', label: 'Repo' },
		{ key: 'total', label: 'Total Tokens', sortable: true, class: 'font-mono' },
		{ key: 'input', label: 'Input', sortable: true, class: 'font-mono' },
		{ key: 'output', label: 'Output', sortable: true, class: 'font-mono' },
		{ key: 'sessions', label: 'Sessions', sortable: true },
		{ key: '_avg', label: 'Avg/Session', sortable: true, class: 'font-mono' }
	];

	const repoRows = $derived.by(() => {
		const d = data;
		if (!d) return [] as Record<string, unknown>[];
		return d.by_repo.map((r) => ({
			...r,
			_avg: r.sessions > 0 ? Math.round(r.total / r.sessions) : 0
		}));
	});

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
		<div class="grid grid-cols-2 gap-4 md:grid-cols-3">
			<StatCard
				label="Cache Read Tokens"
				value={fmtNum(data.cache_read_tokens)}
				icon={BookOpenIcon}
				color="#3b82f6"
			/>
			<StatCard
				label="Cache Write Tokens"
				value={fmtNum(data.cache_write_tokens)}
				icon={BookMarkedIcon}
				color="#8b5cf6"
			/>
			<StatCard
				label="Cache Savings"
				value={'$' + data.cache_savings_usd.toFixed(2)}
				icon={PiggyBankIcon}
				color="#10b981"
			/>
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

		<DataTable
			columns={repoColumns}
			rows={repoRows}
			searchKeys={['repo']}
			defaultSort="total"
			rowIdKey="repo"
		>
			{#snippet children({ row, col })}
				{#if col.key === 'total' || col.key === 'input' || col.key === 'output' || col.key === '_avg'}
					{fmtNum(row[col.key] as number)}
				{:else}
					{row[col.key] ?? '-'}
				{/if}
			{/snippet}
		</DataTable>

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
