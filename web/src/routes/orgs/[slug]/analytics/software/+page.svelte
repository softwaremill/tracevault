<script lang="ts">
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import StatCard from '$lib/components/StatCard.svelte';
	import HelpTip from '$lib/components/HelpTip.svelte';
	import DataTable from '$lib/components/DataTable.svelte';
	import WrenchIcon from '@lucide/svelte/icons/wrench';
	import TrophyIcon from '@lucide/svelte/icons/trophy';

	const COLORS = [
		'#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6',
		'#ec4899', '#06b6d4', '#84cc16', '#f97316', '#6366f1'
	];

	interface OrgTopTool {
		name: string;
		count: number;
		users: number;
	}

	interface SoftwareResponse {
		org_top_tools: OrgTopTool[];
	}

	let data: SoftwareResponse | null = $state(null);
	let loading = $state(true);
	let error = $state('');

	const slug = $derived($page.params.slug);

	async function fetchData(search: string) {
		loading = true;
		error = '';
		try {
			data = await api.get<SoftwareResponse>(
				`/api/v1/orgs/${slug}/analytics/software` + (search ? '?' + search : '')
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

	const totalUniqueTools = $derived.by(() => {
		if (!data) return 0;
		return data.org_top_tools.length;
	});

	const mostPopularTool = $derived.by(() => {
		if (!data || data.org_top_tools.length === 0) return '-';
		return `${data.org_top_tools[0].name} (${fmtNum(data.org_top_tools[0].count)})`;
	});

	const topToolEntries = $derived.by(() => {
		if (!data) return [];
		return data.org_top_tools.slice(0, 12).map((t, i) => ({
			name: t.name,
			count: t.count,
			users: t.users,
			color: COLORS[i % COLORS.length]
		}));
	});

	const topToolsTotal = $derived(topToolEntries.reduce((s, e) => s + e.count, 0));

	const tableColumns = [
		{ key: 'name', label: 'Software' },
		{ key: 'count', label: 'Total Usage', sortable: true },
		{ key: 'users', label: 'Users', sortable: true }
	];
</script>

<svelte:head>
	<title>Software Analytics - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	<h1 class="text-xl font-semibold">Software Analytics</h1>

	{#if loading}
		<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm">
			<span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"></span>
			Loading...
		</div>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if data}
		<div class="grid grid-cols-2 gap-3">
			<StatCard
				label="Total Unique Tools"
				value={String(totalUniqueTools)}
				icon={WrenchIcon}
				color="#3b82f6"
				tooltip="Number of distinct CLI tools detected across all users."
			/>
			<StatCard
				label="Most Popular Tool"
				value={mostPopularTool}
				icon={TrophyIcon}
				color="#f59e0b"
				tooltip="The CLI tool with the highest total usage count."
			/>
		</div>

		<div class="border-border rounded-lg border p-3">
			<h4 class="mb-2 text-sm font-semibold">
				Top Tools (org-wide) <HelpTip text="Most frequently used CLI tools across all users." />
			</h4>
			{#if topToolEntries.length > 0}
				<div class="flex h-9 overflow-hidden rounded-md">
					{#each topToolEntries as entry}
						<div
							class="flex items-center justify-center overflow-hidden text-xs font-semibold text-white transition-all hover:brightness-110"
							style="flex: {entry.count}; background: {entry.color}"
							title="{entry.name}: {fmtNum(entry.count)} ({entry.users} users)"
						>
							{#if topToolsTotal > 0 && entry.count / topToolsTotal > 0.06}
								<span class="truncate px-1">{entry.name}</span>
							{/if}
						</div>
					{/each}
				</div>
				<div class="mt-2 flex flex-wrap gap-x-4 gap-y-1">
					{#each topToolEntries as entry}
						<div class="text-muted-foreground flex items-center gap-1.5 text-xs">
							<span class="inline-block h-2.5 w-2.5 rounded-sm" style="background: {entry.color}"></span>
							{entry.name}
							<span class="text-muted-foreground/60">{fmtNum(entry.count)}</span>
						</div>
					{/each}
				</div>
			{:else}
				<p class="text-muted-foreground text-sm">No data</p>
			{/if}
		</div>

		<DataTable
			columns={tableColumns}
			rows={data.org_top_tools}
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
