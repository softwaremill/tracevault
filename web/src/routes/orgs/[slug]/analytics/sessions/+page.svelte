<script lang="ts">
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import DataTable from '$lib/components/DataTable.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import Chart from '$lib/components/chart.svelte';
	import SessionDetailPanel from '$lib/components/session-detail/SessionDetailPanel.svelte';
	import MonitorPlayIcon from '@lucide/svelte/icons/monitor-play';
	import ClockIcon from '@lucide/svelte/icons/clock';
	import MessageSquareIcon from '@lucide/svelte/icons/message-square';
	import DollarSignIcon from '@lucide/svelte/icons/dollar-sign';
	import CpuIcon from '@lucide/svelte/icons/cpu';
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

	interface SessionItem {
		id: string;
		session_id: string;
		model: string | null;
		duration_ms: number | null;
		started_at: string | null;
		ended_at: string | null;
		user_messages: number | null;
		assistant_messages: number | null;
		tool_calls: Record<string, number> | null;
		total_tool_calls: number | null;
		total_tokens: number | null;
		estimated_cost_usd: number | null;
		compactions: number | null;
		commit_sha: string;
		author: string;
		repo_name: string;
	}

	interface SessionsResponse {
		sessions: SessionItem[];
		tool_frequency: Record<string, number>;
		avg_duration_ms: number | null;
		avg_messages_per_session: number | null;
		total_sessions: number;
	}

	let data: SessionsResponse | null = $state(null);
	let loading = $state(true);
	let error = $state('');
	let expandedSessionId = $state<string | null>(null);

	const slug = $derived($page.params.slug);

	const tableColumns = [
		{ key: 'session_id', label: 'Session ID' },
		{ key: 'repo_name', label: 'Repo', sortable: true },
		{ key: 'author', label: 'Author', sortable: true },
		{ key: 'duration_ms', label: 'Duration', sortable: true },
		{ key: '_messages', label: 'Messages', sortable: true },
		{ key: 'total_tool_calls', label: 'Tool Calls', sortable: true },
		{ key: 'estimated_cost_usd', label: 'Cost', sortable: true },
		{ key: 'model', label: 'Model' },
		{ key: 'started_at', label: 'Started', sortable: true }
	];

	const tableRows = $derived.by(() => {
		const d = data;
		if (!d) return [] as Record<string, unknown>[];
		return d.sessions.map((s) => ({
			...s,
			_messages: (s.user_messages ?? 0) + (s.assistant_messages ?? 0)
		}));
	});

	const totalCost = $derived.by(() => {
		const d = data;
		if (!d) return 0;
		return d.sessions.reduce((sum: number, s) => sum + (s.estimated_cost_usd ?? 0), 0);
	});

	const topModel = $derived.by(() => {
		const d = data;
		const freq: Record<string, number> = {};
		for (const s of d?.sessions ?? []) {
			if (s.model) freq[s.model] = (freq[s.model] ?? 0) + 1;
		}
		let best = '-';
		let bestCount = 0;
		for (const [model, count] of Object.entries(freq)) {
			if (count > bestCount) {
				best = model;
				bestCount = count;
			}
		}
		return best;
	});

	async function fetchData(search: string) {
		loading = true;
		error = '';
		try {
			data = await api.get<SessionsResponse>(`/api/v1/orgs/${slug}/analytics/sessions` + (search ? '?' + search : ''));
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

	function fmtCost(n: number | null): string {
		if (n == null) return '-';
		return `$${n.toFixed(2)}`;
	}

	function fmtNum(n: number): string {
		if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
		if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
		return String(n);
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

	function toolFrequencyChartData(d: SessionsResponse) {
		const entries = Object.entries(d.tool_frequency).sort((a, b) => b[1] - a[1]).slice(0, 10);
		return {
			labels: entries.map(([k]) => k),
			datasets: [
				{
					data: entries.map(([, v]) => v),
					backgroundColor: COLORS.slice(0, entries.length).concat(
						Array(Math.max(0, entries.length - COLORS.length)).fill('#94a3b8')
					)
				}
			]
		};
	}
</script>

<svelte:head>
	<title>Session Analytics - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	<h1 class="text-2xl font-bold">Session Analytics</h1>

	{#if loading}
		<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm">
			<span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"></span>
			Loading...
		</div>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if data}
		<!-- Stat cards -->
		<div class="grid grid-cols-2 gap-4 md:grid-cols-3 lg:grid-cols-5">
			<StatCard label="Total Sessions" value={fmtNum(data.total_sessions)} icon={MonitorPlayIcon} color="#3b82f6" />
			<StatCard label="Avg Duration" value={fmtDuration(data.avg_duration_ms)} icon={ClockIcon} color="#10b981" />
			<StatCard
				label="Avg Messages/Session"
				value={data.avg_messages_per_session != null ? data.avg_messages_per_session.toFixed(1) : '-'}
				icon={MessageSquareIcon}
				color="#f59e0b"
			/>
			<StatCard label="Total Cost" value={fmtCost(totalCost)} icon={DollarSignIcon} color="#dc2626" />
			<StatCard label="Top Model" value={topModel} icon={CpuIcon} color="#8b5cf6" />
		</div>

		<!-- Tool Frequency chart -->
		<div class="border-border rounded-lg border p-3">
			<h4 class="mb-2 text-sm font-semibold">Tool Frequency</h4>
			<div class="flex justify-center">
				{#if Object.keys(data.tool_frequency).length > 0}
					<div class="max-w-[400px]">
						<Chart
							type="doughnut"
							data={toolFrequencyChartData(data)}
							options={{ responsive: true, plugins: { legend: { position: 'bottom' } } }}
						/>
					</div>
				{:else}
					<p class="text-muted-foreground text-sm">No tool data</p>
				{/if}
			</div>
		</div>

		<!-- Sessions table -->
		<DataTable
			columns={tableColumns}
			rows={tableRows}
			searchKeys={['session_id', 'repo_name', 'author', 'model']}
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
				{:else if col.key === '_messages'}
					<span class="font-mono">{row._messages}</span>
				{:else if col.key === 'total_tool_calls'}
					<span class="font-mono">{row.total_tool_calls ?? 0}</span>
				{:else if col.key === 'estimated_cost_usd'}
					<span class="font-mono">{fmtCost(row.estimated_cost_usd as number | null)}</span>
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
	{/if}
</div>
