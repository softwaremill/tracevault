<script lang="ts">
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import StatCard from '$lib/components/StatCard.svelte';
	import HelpTip from '$lib/components/HelpTip.svelte';
	import DataTable from '$lib/components/DataTable.svelte';
	import SessionDetailPanel from '$lib/components/session-detail/SessionDetailPanel.svelte';
	import MonitorPlayIcon from '@lucide/svelte/icons/monitor-play';
	import CoinsIcon from '@lucide/svelte/icons/coins';
	import DollarSignIcon from '@lucide/svelte/icons/dollar-sign';
	import ClockIcon from '@lucide/svelte/icons/clock';
	import WrenchIcon from '@lucide/svelte/icons/wrench';

	const COLORS = ['#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899', '#06b6d4', '#84cc16'];

	interface ModelPref {
		model: string;
		sessions: number;
	}
	interface RecentSession {
		id: string;
		session_id: string;
		repo_name: string;
		started_at: string | null;
		duration_ms: number | null;
		cost_usd: number | null;
		model: string | null;
	}
	interface AuthorDetailResponse {
		user: { user_id: string; email: string; name: string | null };
		sessions: number;
		tokens: number;
		cost_usd: number;
		avg_duration_ms: number | null;
		total_tool_calls: number;
		model_preferences: ModelPref[];
		top_software: string[];
		recent_sessions: RecentSession[];
	}

	let data: AuthorDetailResponse | null = $state(null);
	let loading = $state(true);
	let error = $state('');
	let expandedSessionId = $state<string | null>(null);

	const slug = $derived($page.params.slug);
	const userId = $derived($page.params.user_id);

	async function fetchData(search: string) {
		loading = true;
		error = '';
		try {
			data = await api.get<AuthorDetailResponse>(
				`/api/v1/orgs/${slug}/analytics/authors/${userId}` + (search ? '?' + search : '')
			);
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

	function fmtCost(n: number | null): string {
		if (n == null) return '-';
		return `$${n.toFixed(2)}`;
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

	function fmtRelativeTime(iso: string | null): string {
		if (!iso) return '-';
		const diff = Date.now() - new Date(iso).getTime();
		const minutes = Math.floor(diff / 60000);
		const hours = Math.floor(minutes / 60);
		const days = Math.floor(hours / 24);
		if (days > 0) return `${days}d ago`;
		if (hours > 0) return `${hours}h ago`;
		if (minutes > 0) return `${minutes}m ago`;
		return 'just now';
	}

	const sessionColumns = [
		{ key: 'session_id', label: 'Session ID' },
		{ key: 'repo_name', label: 'Repo', sortable: true },
		{ key: 'duration_ms', label: 'Duration', sortable: true },
		{ key: 'cost_usd', label: 'Cost', sortable: true },
		{ key: 'model', label: 'Model' },
		{ key: 'started_at', label: 'Started', sortable: true }
	];
</script>

<svelte:head>
	<title>{data ? (data.user.name ?? data.user.email) : 'User'} - Author Analytics - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	{#if loading}
		<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm">
			<span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"></span>
			Loading...
		</div>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if data}
		<div class="flex items-center gap-3">
			<a href="/orgs/{slug}/analytics/authors" class="text-muted-foreground hover:text-foreground text-sm">&larr; Back</a>
			<h1 class="text-xl font-semibold">{data.user.name ?? data.user.email}</h1>
			{#if data.user.name}
				<span class="text-muted-foreground text-sm">{data.user.email}</span>
			{/if}
		</div>

		<div class="grid grid-cols-2 gap-3 md:grid-cols-3 lg:grid-cols-5">
			<StatCard label="Sessions" value={fmtNum(data.sessions)} icon={MonitorPlayIcon} color="#3b82f6" tooltip="Total AI coding sessions." />
			<StatCard label="Tokens" value={fmtNum(data.tokens)} icon={CoinsIcon} color="#f59e0b" tooltip="Total tokens consumed." />
			<StatCard label="Cost" value={fmtCost(data.cost_usd)} icon={DollarSignIcon} color="#dc2626" tooltip="Total estimated cost." />
			<StatCard label="Avg Duration" value={fmtDuration(data.avg_duration_ms)} icon={ClockIcon} color="#06b6d4" tooltip="Average session duration." />
			<StatCard label="Tool Calls" value={fmtNum(data.total_tool_calls)} icon={WrenchIcon} color="#10b981" tooltip="Total tool invocations." />
		</div>

		{#if data.model_preferences.length > 0}
			<div class="border-border rounded-lg border p-3">
				<h4 class="mb-2 text-sm font-semibold">Model Preferences</h4>
				<div class="flex flex-wrap gap-2">
					{#each data.model_preferences as pref}
						<span class="rounded-full px-3 py-1 text-xs font-medium" style="background: rgba(167,139,250,0.12); color: #a78bfa; border: 1px solid rgba(167,139,250,0.25)">{pref.model} ({pref.sessions})</span>
					{/each}
				</div>
			</div>
		{/if}

		{#if data.top_software.length > 0}
			<div class="border-border rounded-lg border p-3">
				<div class="flex items-center justify-between mb-2">
					<h4 class="text-sm font-semibold">Top Software <HelpTip text="Most frequently used CLI tools." /></h4>
					<a href="/orgs/{slug}/analytics/software/users/{userId}" class="text-xs text-primary hover:underline">View all software &rarr;</a>
				</div>
				<div class="flex flex-wrap gap-2">
					{#each data.top_software as tool, i}
						<span class="rounded-full px-3 py-1 text-xs font-medium" style="background: {COLORS[i % COLORS.length]}22; color: {COLORS[i % COLORS.length]}; border: 1px solid {COLORS[i % COLORS.length]}40">{tool}</span>
					{/each}
				</div>
			</div>
		{/if}

		<div>
			<h4 class="mb-2 text-sm font-semibold">Recent Sessions <HelpTip text="Latest coding sessions." /></h4>
			<DataTable
				columns={sessionColumns}
				rows={data.recent_sessions}
				searchKeys={['session_id', 'repo_name']}
				defaultSort="started_at"
				defaultSortDir="desc"
				rowIdKey="id"
				onRowClick={(row) => {
					const id = row.id as string;
					expandedSessionId = expandedSessionId === id ? null : id;
				}}
				expandedRowId={expandedSessionId}
			>
				{#snippet children({ row, col })}
					{#if col.key === 'session_id'}
						<span class="font-mono">{(row.session_id as string).slice(0, 8)}</span>
					{:else if col.key === 'duration_ms'}
						<span class="font-mono">{fmtDuration(row.duration_ms as number | null)}</span>
					{:else if col.key === 'cost_usd'}
						<span class="font-mono">{fmtCost(row.cost_usd as number | null)}</span>
					{:else if col.key === 'model'}
						{#if row.model}
							<span class="rounded-full px-2 py-0.5 text-[10px]" style="background: rgba(79,110,247,0.12); color: #4f6ef7; border: 1px solid rgba(79,110,247,0.25)">{row.model}</span>
						{:else}
							<span class="text-muted-foreground">-</span>
						{/if}
					{:else if col.key === 'started_at'}
						{fmtRelativeTime(row.started_at as string | null)}
					{:else}
						{row[col.key] ?? '-'}
					{/if}
				{/snippet}
				{#snippet expandedRow({ row })}
					<SessionDetailPanel sessionId={row.id as string} />
				{/snippet}
			</DataTable>
		</div>
	{/if}
</div>
