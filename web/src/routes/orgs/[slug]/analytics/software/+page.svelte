<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { api } from '$lib/api';
	import StatCard from '$lib/components/StatCard.svelte';
	import HelpTip from '$lib/components/HelpTip.svelte';
	import DataTable from '$lib/components/DataTable.svelte';
	import Chart from '$lib/components/chart.svelte';
	import WrenchIcon from '@lucide/svelte/icons/wrench';
	import TrophyIcon from '@lucide/svelte/icons/trophy';
	import UsersIcon from '@lucide/svelte/icons/users';
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
		'#3b82f6',
		'#10b981',
		'#f59e0b',
		'#ef4444',
		'#8b5cf6',
		'#ec4899',
		'#06b6d4',
		'#84cc16',
		'#f97316',
		'#6366f1'
	];

	interface SoftwareUser {
		user_id: string;
		email: string;
		name: string | null;
		unique_tools: number;
		total_usage: number;
		top_tools: string[];
		last_active: string;
	}

	interface OrgTopTool {
		name: string;
		count: number;
		users: number;
	}

	interface SoftwareResponse {
		users: SoftwareUser[];
		total_users: number;
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

	const totalUniqueTools = $derived(
		data ? new Set(data.org_top_tools.map((t) => t.name)).size : 0
	);

	const mostPopularTool = $derived.by(() => {
		if (!data || data.org_top_tools.length === 0) return '-';
		return `${data.org_top_tools[0].name} (${fmtNum(data.org_top_tools[0].count)})`;
	});

	const mostDiverseUser = $derived.by(() => {
		if (!data || data.users.length === 0) return '-';
		const sorted = [...data.users].sort((a, b) => b.unique_tools - a.unique_tools);
		return sorted[0].email;
	});

	const tableColumns = [
		{ key: 'email', label: 'User' },
		{ key: 'unique_tools', label: 'Unique Tools', sortable: true },
		{ key: 'total_usage', label: 'Total Usage', sortable: true },
		{ key: '_top_tools', label: 'Top Tools' },
		{ key: 'last_active', label: 'Last Active', sortable: true }
	];

	const tableRows = $derived.by(() => {
		if (!data) return [] as Record<string, unknown>[];
		return data.users.map((u) => ({
			...u,
			_top_tools: u.top_tools.join(', ')
		}));
	});

	function topToolsChartData(d: SoftwareResponse) {
		const tools = d.org_top_tools.slice(0, 15);
		return {
			labels: tools.map((t) => t.name),
			datasets: [
				{
					label: 'Usage Count',
					data: tools.map((t) => t.count),
					backgroundColor: tools.map((_, i) => COLORS[i % COLORS.length])
				}
			]
		};
	}
</script>

<svelte:head>
	<title>Software Analytics - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	<h1 class="text-2xl font-bold">Software Analytics</h1>

	{#if loading}
		<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm">
			<span
				class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
			></span>
			Loading...
		</div>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if data}
		<div class="grid grid-cols-2 gap-4 md:grid-cols-3">
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
			<StatCard
				label="Most Diverse User"
				value={mostDiverseUser}
				icon={UsersIcon}
				color="#8b5cf6"
				tooltip="The user who has used the most distinct CLI tools."
			/>
		</div>

		<div class="border-border rounded-lg border p-3">
			<h4 class="mb-2 text-sm font-semibold">
				Top Tools (org-wide)<HelpTip
					text="Most frequently used CLI tools across all users in the organization."
				/>
			</h4>
			{#if data.org_top_tools.length > 0}
				<Chart
					type="bar"
					data={topToolsChartData(data)}
					options={{
						responsive: true,
						indexAxis: 'y',
						plugins: { legend: { display: false } }
					}}
				/>
			{:else}
				<p class="text-muted-foreground text-sm">No data</p>
			{/if}
		</div>

		<DataTable
			columns={tableColumns}
			rows={tableRows}
			searchKeys={['email', 'name']}
			defaultSort="total_usage"
			defaultSortDir="desc"
			rowIdKey="user_id"
			onRowClick={(row) => {
				goto(`/orgs/${slug}/analytics/software/users/${row.user_id}`);
			}}
		>
			{#snippet children({ row, col })}
				{#if col.key === 'email'}
					<span class="font-medium">{row.name ?? row.email}</span>
					{#if row.name}
						<span class="text-muted-foreground ml-1 text-[10px]">{row.email}</span>
					{/if}
				{:else if col.key === 'unique_tools'}
					<span class="font-mono">{row.unique_tools}</span>
				{:else if col.key === 'total_usage'}
					<span class="font-mono">{fmtNum(row.total_usage as number)}</span>
				{:else if col.key === '_top_tools'}
					<div class="flex flex-wrap gap-1">
						{#each row.top_tools as tool}
							<span
								class="rounded-full px-2 py-0.5 text-[10px]"
								style="background: rgba(59,130,246,0.12); color: #3b82f6; border: 1px solid rgba(59,130,246,0.25)"
								>{tool}</span
							>
						{/each}
					</div>
				{:else if col.key === 'last_active'}
					{fmtRelativeTime(row.last_active as string)}
				{:else}
					{row[col.key] ?? '-'}
				{/if}
			{/snippet}
		</DataTable>
	{/if}
</div>
