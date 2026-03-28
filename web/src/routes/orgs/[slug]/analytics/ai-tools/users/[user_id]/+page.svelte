<script lang="ts">
	import { page } from '$app/stores';
	import { useFetch } from '$lib/hooks/use-fetch.svelte';
	import { fmtNum, fmtDuration, fmtRelativeTime } from '$lib/utils/format';
	import StatCard from '$lib/components/StatCard.svelte';
	import HelpTip from '$lib/components/HelpTip.svelte';
	import DataTable from '$lib/components/DataTable.svelte';
	import Chart from '$lib/components/chart.svelte';
	import LoadingState from '$lib/components/LoadingState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import SessionDetailPanel from '$lib/components/session-detail/SessionDetailPanel.svelte';
	import { formatDate } from '$lib/utils/date';
	import BotIcon from '@lucide/svelte/icons/bot';
	import SparklesIcon from '@lucide/svelte/icons/sparkles';
	import {
		Chart as ChartJS,
		CategoryScale,
		LinearScale,
		BarElement,
		Title,
		Tooltip,
		Legend
	} from 'chart.js';

	ChartJS.register(CategoryScale, LinearScale, BarElement, Title, Tooltip, Legend);

	const COLORS = [
		'#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6',
		'#ec4899', '#06b6d4', '#84cc16', '#f97316', '#6366f1'
	];

	interface AiToolItem {
		name: string;
		usage_count: number;
		first_seen: string;
		last_seen: string;
		session_count: number;
	}
	interface RecentSession {
		id: string;
		session_id: string;
		repo_name: string;
		started_at: string | null;
		duration_ms: number | null;
		ai_tools_used: string[];
	}
	interface AiToolsUserDetailResponse {
		user: { user_id: string; email: string; name: string | null };
		mcp_servers: AiToolItem[];
		skill_groups: AiToolItem[];
		recent_sessions: RecentSession[];
	}

	let expandedSessionId = $state<string | null>(null);

	const slug = $derived($page.params.slug);
	const userId = $derived($page.params.user_id);
	const search = $derived($page.url.search.replace(/^\?/, ''));

	const aiToolsUserQuery = useFetch<AiToolsUserDetailResponse>(
		() => `/api/v1/orgs/${slug}/analytics/ai-tools/users/${userId}` + (search ? '?' + search : '')
	);

	const toolColumns = [
		{ key: 'name', label: 'Name' },
		{ key: 'usage_count', label: 'Usage Count', sortable: true },
		{ key: 'session_count', label: 'Sessions', sortable: true },
		{ key: 'first_seen', label: 'First Seen', sortable: true },
		{ key: 'last_seen', label: 'Last Seen', sortable: true }
	];

	const sessionColumns = [
		{ key: 'session_id', label: 'Session ID' },
		{ key: 'repo_name', label: 'Repo', sortable: true },
		{ key: 'duration_ms', label: 'Duration', sortable: true },
		{ key: '_tools', label: 'AI Tools Used' },
		{ key: 'started_at', label: 'Started', sortable: true }
	];

	const sessionRows = $derived.by(() => {
		if (!aiToolsUserQuery.data) return [] as Record<string, unknown>[];
		return aiToolsUserQuery.data.recent_sessions.map((s) => ({
			...s,
			_tools: s.ai_tools_used.join(', ')
		}));
	});

	function chartData(items: AiToolItem[]) {
		const top = items.slice(0, 10);
		return {
			labels: top.map((s) => s.name),
			datasets: [
				{
					label: 'Usage Count',
					data: top.map((s) => s.usage_count),
					backgroundColor: top.map((_, i) => COLORS[i % COLORS.length])
				}
			]
		};
	}
</script>

<svelte:head>
	<title>{aiToolsUserQuery.data ? (aiToolsUserQuery.data.user.name ?? aiToolsUserQuery.data.user.email) : 'User'} - AI Tools - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	{#if aiToolsUserQuery.loading}
		<LoadingState />
	{:else if aiToolsUserQuery.error}
		<ErrorState message={aiToolsUserQuery.error} onRetry={aiToolsUserQuery.refetch} />
	{:else if aiToolsUserQuery.data}
		{@const data = aiToolsUserQuery.data}
		<div class="flex items-center gap-3">
			<a href="/orgs/{slug}/analytics/ai-tools" class="text-muted-foreground hover:text-foreground text-sm">&larr; Back</a>
			<h1 class="text-xl font-semibold">{data.user.name ?? data.user.email}</h1>
			{#if data.user.name}
				<span class="text-muted-foreground text-sm">{data.user.email}</span>
			{/if}
		</div>

		<div class="grid grid-cols-2 gap-3">
			<StatCard
				label="MCP Servers"
				value={String(data.mcp_servers.length)}
				icon={BotIcon}
				color="#3b82f6"
				tooltip="Number of distinct MCP servers this user has used."
			/>
			<StatCard
				label="Skill Groups"
				value={String(data.skill_groups.length)}
				icon={SparklesIcon}
				color="#8b5cf6"
				tooltip="Number of distinct skill groups this user has used."
			/>
		</div>

		<!-- MCP Servers -->
		{#if data.mcp_servers.length > 0}
			<div class="grid gap-6 lg:grid-cols-2">
				<div class="border-border rounded-lg border p-3">
					<h4 class="mb-2 text-sm font-semibold">Top MCP Servers <HelpTip text="Most frequently used MCP servers by this user." /></h4>
					<Chart
						type="bar"
						data={chartData(data.mcp_servers)}
						options={{ responsive: true, indexAxis: 'y', plugins: { legend: { display: false } } }}
					/>
				</div>
				<div>
					<h4 class="mb-2 text-sm font-semibold">All MCP Servers <HelpTip text="Complete list of MCP servers detected." /></h4>
					<DataTable
						columns={toolColumns}
						rows={data.mcp_servers}
						searchKeys={['name']}
						defaultSort="usage_count"
						defaultSortDir="desc"
						rowIdKey="name"
					>
						{#snippet children({ row, col })}
							{#if col.key === 'name'}
								<span class="font-mono font-medium">{row.name}</span>
							{:else if col.key === 'usage_count'}
								<span class="font-mono">{fmtNum(row.usage_count as number)}</span>
							{:else if col.key === 'session_count'}
								<span class="font-mono">{row.session_count}</span>
							{:else if col.key === 'first_seen'}
								{formatDate(row.first_seen as string)}
							{:else if col.key === 'last_seen'}
								{formatDate(row.last_seen as string)}
							{:else}
								{row[col.key] ?? '-'}
							{/if}
						{/snippet}
					</DataTable>
				</div>
			</div>
		{/if}

		<!-- Skill Groups -->
		{#if data.skill_groups.length > 0}
			<div class="grid gap-6 lg:grid-cols-2">
				<div class="border-border rounded-lg border p-3">
					<h4 class="mb-2 text-sm font-semibold">Top Skill Groups <HelpTip text="Most frequently used skill groups by this user." /></h4>
					<Chart
						type="bar"
						data={chartData(data.skill_groups)}
						options={{ responsive: true, indexAxis: 'y', plugins: { legend: { display: false } } }}
					/>
				</div>
				<div>
					<h4 class="mb-2 text-sm font-semibold">All Skill Groups <HelpTip text="Complete list of skill groups detected." /></h4>
					<DataTable
						columns={toolColumns}
						rows={data.skill_groups}
						searchKeys={['name']}
						defaultSort="usage_count"
						defaultSortDir="desc"
						rowIdKey="name"
					>
						{#snippet children({ row, col })}
							{#if col.key === 'name'}
								<span class="font-mono font-medium">{row.name}</span>
							{:else if col.key === 'usage_count'}
								<span class="font-mono">{fmtNum(row.usage_count as number)}</span>
							{:else if col.key === 'session_count'}
								<span class="font-mono">{row.session_count}</span>
							{:else if col.key === 'first_seen'}
								{formatDate(row.first_seen as string)}
							{:else if col.key === 'last_seen'}
								{formatDate(row.last_seen as string)}
							{:else}
								{row[col.key] ?? '-'}
							{/if}
						{/snippet}
					</DataTable>
				</div>
			</div>
		{/if}

		<!-- Recent Sessions -->
		<div>
			<h4 class="mb-2 text-sm font-semibold">Recent Sessions <HelpTip text="Latest sessions with AI tools detected." /></h4>
			<DataTable
				columns={sessionColumns}
				rows={sessionRows}
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
					{:else if col.key === '_tools'}
						<div class="flex flex-wrap gap-1">
							{#each (row.ai_tools_used as string[]) as tool}
								<span class="rounded-full px-2 py-0.5 text-[10px]" style="background: rgba(139,92,246,0.12); color: #8b5cf6; border: 1px solid rgba(139,92,246,0.25)">{tool}</span>
							{/each}
						</div>
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
