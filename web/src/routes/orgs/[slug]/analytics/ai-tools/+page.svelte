<script lang="ts">
	import { page } from '$app/stores';
	import { useFetch } from '$lib/hooks/use-fetch.svelte';
	import { fmtNum } from '$lib/utils/format';
	import StatCard from '$lib/components/StatCard.svelte';
	import HelpTip from '$lib/components/HelpTip.svelte';
	import DataTable from '$lib/components/DataTable.svelte';
	import LoadingState from '$lib/components/LoadingState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import BotIcon from '@lucide/svelte/icons/bot';
	import SparklesIcon from '@lucide/svelte/icons/sparkles';
	import ActivityIcon from '@lucide/svelte/icons/activity';

	const COLORS = [
		'#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6',
		'#ec4899', '#06b6d4', '#84cc16', '#f97316', '#6366f1'
	];

	interface AiToolEntry {
		name: string;
		count: number;
		users: number;
	}

	interface AiToolsResponse {
		mcp_servers: AiToolEntry[];
		skill_groups: AiToolEntry[];
	}

	const slug = $derived($page.params.slug);
	const search = $derived($page.url.search.replace(/^\?/, ''));

	const aiToolsQuery = useFetch<AiToolsResponse>(
		() => `/api/v1/orgs/${slug}/analytics/ai-tools` + (search ? '?' + search : '')
	);

	const totalMcpServers = $derived.by(() => {
		if (!aiToolsQuery.data) return 0;
		return aiToolsQuery.data.mcp_servers.length;
	});
	const totalSkillGroups = $derived.by(() => {
		if (!aiToolsQuery.data) return 0;
		return aiToolsQuery.data.skill_groups.length;
	});
	const totalUsage = $derived.by(() => {
		if (!aiToolsQuery.data) return 0;
		const mcp = aiToolsQuery.data.mcp_servers.reduce((s, e) => s + e.count, 0);
		const skill = aiToolsQuery.data.skill_groups.reduce((s, e) => s + e.count, 0);
		return mcp + skill;
	});

	function barEntries(items: AiToolEntry[]) {
		return items.slice(0, 12).map((t, i) => ({
			name: t.name,
			count: t.count,
			users: t.users,
			color: COLORS[i % COLORS.length]
		}));
	}

	const mcpBarEntries = $derived.by(() => {
		if (!aiToolsQuery.data) return [];
		return barEntries(aiToolsQuery.data.mcp_servers);
	});
	const skillBarEntries = $derived.by(() => {
		if (!aiToolsQuery.data) return [];
		return barEntries(aiToolsQuery.data.skill_groups);
	});
	const mcpBarTotal = $derived(mcpBarEntries.reduce((s, e) => s + e.count, 0));
	const skillBarTotal = $derived(skillBarEntries.reduce((s, e) => s + e.count, 0));

	const tableColumns = [
		{ key: 'name', label: 'Name' },
		{ key: 'count', label: 'Total Usage', sortable: true },
		{ key: 'users', label: 'Users', sortable: true }
	];
</script>

<svelte:head>
	<title>AI Tools Analytics - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	<h1 class="text-xl font-semibold">AI Tools Analytics</h1>

	{#if aiToolsQuery.loading}
		<LoadingState />
	{:else if aiToolsQuery.error}
		<ErrorState message={aiToolsQuery.error} onRetry={aiToolsQuery.refetch} />
	{:else if aiToolsQuery.data}
		{@const data = aiToolsQuery.data}
		<div class="grid grid-cols-3 gap-3">
			<StatCard
				label="MCP Servers"
				value={String(totalMcpServers)}
				icon={BotIcon}
				color="#3b82f6"
				tooltip="Number of distinct MCP servers detected across all users."
			/>
			<StatCard
				label="Skill Groups"
				value={String(totalSkillGroups)}
				icon={SparklesIcon}
				color="#8b5cf6"
				tooltip="Number of distinct skill groups (namespaces) detected."
			/>
			<StatCard
				label="Total Usage"
				value={fmtNum(totalUsage)}
				icon={ActivityIcon}
				color="#10b981"
				tooltip="Total invocations of MCP tools and skills combined."
			/>
		</div>

		<!-- MCP Servers Section -->
		<div class="border-border rounded-lg border p-3">
			<h4 class="mb-2 text-sm font-semibold">
				MCP Servers <HelpTip text="MCP servers used across all sessions, aggregated by server name." />
			</h4>
			{#if mcpBarEntries.length > 0}
				<div class="flex h-9 overflow-hidden rounded-md">
					{#each mcpBarEntries as entry}
						<div
							class="flex items-center justify-center overflow-hidden text-xs font-semibold text-white transition-all hover:brightness-110"
							style="flex: {entry.count}; background: {entry.color}"
							title="{entry.name}: {fmtNum(entry.count)} ({entry.users} users)"
						>
							{#if mcpBarTotal > 0 && entry.count / mcpBarTotal > 0.06}
								<span class="truncate px-1">{entry.name}</span>
							{/if}
						</div>
					{/each}
				</div>
				<div class="mt-2 flex flex-wrap gap-x-4 gap-y-1">
					{#each mcpBarEntries as entry}
						<div class="text-muted-foreground flex items-center gap-1.5 text-xs">
							<span class="inline-block h-2.5 w-2.5 rounded-sm" style="background: {entry.color}"></span>
							{entry.name}
							<span class="text-muted-foreground/60">{fmtNum(entry.count)}</span>
						</div>
					{/each}
				</div>
			{:else}
				<p class="text-muted-foreground text-sm">No MCP server usage recorded yet.</p>
			{/if}
		</div>

		<DataTable
			columns={tableColumns}
			rows={data.mcp_servers}
			searchKeys={['name']}
			defaultSort="count"
			defaultSortDir="desc"
			rowIdKey="name"
		>
			{#snippet children({ row, col })}
				{#if col.key === 'name'}
					<span class="font-mono font-medium">{row.name}</span>
				{:else if col.key === 'count'}
					<span class="font-mono">{fmtNum(row.count as number)}</span>
				{:else if col.key === 'users'}
					<span class="font-mono">{row.users}</span>
				{:else}
					{row[col.key] ?? '-'}
				{/if}
			{/snippet}
		</DataTable>

		<!-- Skill Groups Section -->
		<div class="border-border rounded-lg border p-3">
			<h4 class="mb-2 text-sm font-semibold">
				Skill Groups <HelpTip text="Skill namespaces used across all sessions." />
			</h4>
			{#if skillBarEntries.length > 0}
				<div class="flex h-9 overflow-hidden rounded-md">
					{#each skillBarEntries as entry}
						<div
							class="flex items-center justify-center overflow-hidden text-xs font-semibold text-white transition-all hover:brightness-110"
							style="flex: {entry.count}; background: {entry.color}"
							title="{entry.name}: {fmtNum(entry.count)} ({entry.users} users)"
						>
							{#if skillBarTotal > 0 && entry.count / skillBarTotal > 0.06}
								<span class="truncate px-1">{entry.name}</span>
							{/if}
						</div>
					{/each}
				</div>
				<div class="mt-2 flex flex-wrap gap-x-4 gap-y-1">
					{#each skillBarEntries as entry}
						<div class="text-muted-foreground flex items-center gap-1.5 text-xs">
							<span class="inline-block h-2.5 w-2.5 rounded-sm" style="background: {entry.color}"></span>
							{entry.name}
							<span class="text-muted-foreground/60">{fmtNum(entry.count)}</span>
						</div>
					{/each}
				</div>
			{:else}
				<p class="text-muted-foreground text-sm">No skill usage recorded yet.</p>
			{/if}
		</div>

		<DataTable
			columns={tableColumns}
			rows={data.skill_groups}
			searchKeys={['name']}
			defaultSort="count"
			defaultSortDir="desc"
			rowIdKey="name"
		>
			{#snippet children({ row, col })}
				{#if col.key === 'name'}
					<span class="font-mono font-medium">{row.name}</span>
				{:else if col.key === 'count'}
					<span class="font-mono">{fmtNum(row.count as number)}</span>
				{:else if col.key === 'users'}
					<span class="font-mono">{row.users}</span>
				{:else}
					{row[col.key] ?? '-'}
				{/if}
			{/snippet}
		</DataTable>
	{/if}
</div>
