<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { api } from '$lib/api';
	import DataTable from '$lib/components/DataTable.svelte';
	import HelpTip from '$lib/components/HelpTip.svelte';
	import Chart from '$lib/components/chart.svelte';
	import {
		Chart as ChartJS,
		CategoryScale,
		LinearScale,
		PointElement,
		LineElement,
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
		Title,
		Tooltip,
		Legend,
		Filler
	);

	const COLORS = ['#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899', '#06b6d4', '#84cc16'];

	interface AuthorLeaderboard {
		user_id: string;
		author: string;
		sessions: number;
		tokens: number;
		cost: number;
		ai_pct: number | null;
		last_active: string;
		avg_duration_ms: number | null;
		total_tool_calls: number;
	}
	interface AuthorTimeline {
		date: string;
		author: string;
		commits: number;
	}
	interface AuthorModelPreference {
		author: string;
		model: string;
		sessions: number;
	}
	interface AuthorsResponse {
		leaderboard: AuthorLeaderboard[];
		timeline: AuthorTimeline[];
		model_preferences: AuthorModelPreference[];
	}

	let data: AuthorsResponse | null = $state(null);
	let loading = $state(true);
	let error = $state('');

	const slug = $derived($page.params.slug);

	async function fetchData(search: string) {
		loading = true;
		error = '';
		try {
			data = await api.get<AuthorsResponse>(`/api/v1/orgs/${slug}/analytics/authors` + (search ? '?' + search : ''));
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

	function fmtRelativeTime(iso: string): string {
		const diff = Date.now() - new Date(iso).getTime();
		const minutes = Math.floor(diff / 60000);
		const hours = Math.floor(minutes / 60);
		const days = Math.floor(hours / 24);
		if (days > 0) return `${days}d ago`;
		if (hours > 0) return `${hours}h ago`;
		if (minutes > 0) return `${minutes}m ago`;
		return 'just now';
	}

	function timelineChartData(d: AuthorsResponse) {
		const authors = [...new Set(d.timeline.map((t) => t.author))];
		const dates = [...new Set(d.timeline.map((t) => t.date))].sort();
		return {
			labels: dates,
			datasets: authors.map((author, i) => ({
				label: author,
				data: dates.map((date) => d.timeline.find((t) => t.date === date && t.author === author)?.commits ?? 0),
				borderColor: COLORS[i % COLORS.length],
				backgroundColor: COLORS[i % COLORS.length] + '33',
				tension: 0.3
			}))
		};
	}

	function modelPrefsForAuthor(author: string): AuthorModelPreference[] {
		if (!data) return [];
		return data.model_preferences.filter((p) => p.author === author);
	}

	const tableColumns = [
		{ key: 'author', label: 'Author' },
		{ key: 'sessions', label: 'Sessions', sortable: true },
		{ key: 'tokens', label: 'Tokens', sortable: true },
		{ key: 'cost', label: 'Cost', sortable: true },
		{ key: 'ai_pct', label: 'AI %', sortable: true },
		{ key: 'avg_duration_ms', label: 'Avg Duration', sortable: true },
		{ key: 'total_tool_calls', label: 'Tool Calls', sortable: true },
		{ key: 'last_active', label: 'Last Active', sortable: true },
		{ key: '_models', label: 'Models' }
	];

	const tableRows = $derived.by(() => {
		if (!data) return [] as Record<string, unknown>[];
		return data.leaderboard.map((r) => ({
			...r,
			_models: modelPrefsForAuthor(r.author)
		}));
	});
</script>

<svelte:head>
	<title>Author Analytics - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	<h1 class="text-xl font-semibold">Author Analytics</h1>

	{#if loading}
		<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm">
			<span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"></span>
			Loading...
		</div>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if data}
		<DataTable
			columns={tableColumns}
			rows={tableRows}
			searchKeys={['author']}
			defaultSort="sessions"
			defaultSortDir="desc"
			rowIdKey="user_id"
			onRowClick={(row) => {
				goto(`/orgs/${slug}/analytics/authors/${row.user_id}`);
			}}
		>
			{#snippet children({ row, col })}
				{#if col.key === 'author'}
					<span class="font-medium">{row.author}</span>
				{:else if col.key === 'tokens'}
					<span class="font-mono">{fmtNum(row.tokens as number)}</span>
				{:else if col.key === 'cost'}
					<span class="font-mono">${(row.cost as number).toFixed(2)}</span>
				{:else if col.key === 'ai_pct'}
					{row.ai_pct != null ? `${(row.ai_pct as number).toFixed(1)}%` : 'N/A'}
				{:else if col.key === 'avg_duration_ms'}
					<span class="font-mono">{fmtDuration(row.avg_duration_ms as number | null)}</span>
				{:else if col.key === 'total_tool_calls'}
					<span class="font-mono">{fmtNum(row.total_tool_calls as number)}</span>
				{:else if col.key === 'last_active'}
					{fmtRelativeTime(row.last_active as string)}
				{:else if col.key === '_models'}
					<div class="flex flex-wrap gap-1">
						{#each (row._models as AuthorModelPreference[]) as pref}
							<span class="rounded-full px-2 py-0.5 text-[10px]" style="background: rgba(167,139,250,0.12); color: #a78bfa; border: 1px solid rgba(167,139,250,0.25)">{pref.model} ({pref.sessions})</span>
						{/each}
					</div>
				{:else}
					{row[col.key] ?? '-'}
				{/if}
			{/snippet}
		</DataTable>

		<div class="border-border rounded-lg border p-3">
			<h4 class="mb-2 text-sm font-semibold">Activity Timeline <HelpTip text="Commits per author per day." /></h4>
			{#if data.timeline.length > 0}
				<Chart
					type="line"
					data={timelineChartData(data)}
					options={{ responsive: true, plugins: { legend: { position: 'top' } } }}
				/>
			{:else}
				<p class="text-muted-foreground text-sm">No data</p>
			{/if}
		</div>
	{/if}
</div>
