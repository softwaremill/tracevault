<script lang="ts">
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import * as Table from '$lib/components/ui/table/index.js';

	interface BranchItem {
		branch: string;
		tag: string | null;
		commits_count: number;
		sessions_count: number;
		total_cost: number | null;
		status: string;
		last_activity: string | null;
	}

	let branches: BranchItem[] = $state([]);
	let loading = $state(true);
	let error = $state('');
	let sortCol = $state<string>('last_activity');
	let sortDir = $state<'asc' | 'desc'>('desc');

	const slug = $derived($page.params.slug);

	async function fetchData() {
		loading = true;
		error = '';
		try {
			const repoId = $page.url.searchParams.get('repo_id');
			const qs = repoId ? `?repo_id=${repoId}` : '';
			branches = await api.get<BranchItem[]>(`/api/v1/orgs/${slug}/traces/branches${qs}`);
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load branches';
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		fetchData();
	});

	function fmtCost(n: number | null): string {
		if (n == null) return '-';
		return `$${n.toFixed(2)}`;
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

	const STATUS_STYLES: Record<string, { bg: string; color: string; border: string }> = {
		tracked: {
			bg: 'rgba(59,130,246,0.12)',
			color: '#3b82f6',
			border: 'rgba(59,130,246,0.25)'
		},
		merged: {
			bg: 'rgba(34,197,94,0.12)',
			color: '#22c55e',
			border: 'rgba(34,197,94,0.25)'
		},
		tagged: {
			bg: 'rgba(168,85,247,0.12)',
			color: '#a855f6',
			border: 'rgba(168,85,247,0.25)'
		}
	};

	function statusStyle(status: string): { bg: string; color: string; border: string } {
		return (
			STATUS_STYLES[status] ?? {
				bg: 'rgba(148,163,184,0.12)',
				color: '#94a3b8',
				border: 'rgba(148,163,184,0.25)'
			}
		);
	}

	function sortBy(col: string) {
		if (sortCol === col) {
			sortDir = sortDir === 'asc' ? 'desc' : 'asc';
		} else {
			sortCol = col;
			sortDir = 'desc';
		}
	}

	function sortIndicator(col: string): string {
		if (sortCol !== col) return '';
		return sortDir === 'asc' ? ' \u2191' : ' \u2193';
	}

	const sorted = $derived(
		[...branches].sort((a, b) => {
			let av: number | string | null;
			let bv: number | string | null;
			switch (sortCol) {
				case 'branch':
					av = a.branch;
					bv = b.branch;
					break;
				case 'commits_count':
					av = a.commits_count;
					bv = b.commits_count;
					break;
				case 'sessions_count':
					av = a.sessions_count;
					bv = b.sessions_count;
					break;
				case 'total_cost':
					av = a.total_cost;
					bv = b.total_cost;
					break;
				case 'status':
					av = a.status;
					bv = b.status;
					break;
				case 'last_activity':
					av = a.last_activity ?? '';
					bv = b.last_activity ?? '';
					break;
				default:
					av = a.last_activity ?? '';
					bv = b.last_activity ?? '';
			}
			if (av == null && bv == null) return 0;
			if (av == null) return 1;
			if (bv == null) return -1;
			const diff =
				typeof av === 'string' ? av.localeCompare(bv as string) : (av as number) - (bv as number);
			return sortDir === 'asc' ? diff : -diff;
		})
	);
</script>

<svelte:head>
	<title>Branches - TraceVault</title>
</svelte:head>

<div class="space-y-4">
	<h1 class="text-2xl font-bold">Branches</h1>

	{#if loading}
		<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm">
			<span
				class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
			></span>
			Loading...
		</div>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if branches.length === 0}
		<p class="text-muted-foreground">No branches tracked yet.</p>
	{:else}
		<Table.Root class="text-xs">
			<Table.Header>
				<Table.Row>
					<Table.Head>
						<button class="hover:underline" onclick={() => sortBy('branch')}>
							Branch{sortIndicator('branch')}
						</button>
					</Table.Head>
					<Table.Head>
						<button class="hover:underline" onclick={() => sortBy('commits_count')}>
							Commits{sortIndicator('commits_count')}
						</button>
					</Table.Head>
					<Table.Head>
						<button class="hover:underline" onclick={() => sortBy('sessions_count')}>
							Sessions{sortIndicator('sessions_count')}
						</button>
					</Table.Head>
					<Table.Head>
						<button class="hover:underline" onclick={() => sortBy('total_cost')}>
							Cost{sortIndicator('total_cost')}
						</button>
					</Table.Head>
					<Table.Head>
						<button class="hover:underline" onclick={() => sortBy('status')}>
							Status{sortIndicator('status')}
						</button>
					</Table.Head>
					<Table.Head>
						<button class="hover:underline" onclick={() => sortBy('last_activity')}>
							Last Activity{sortIndicator('last_activity')}
						</button>
					</Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each sorted as branch}
					{@const ss = statusStyle(branch.status)}
					<Table.Row class="hover:bg-muted/40 transition-colors">
						<Table.Cell>
							<span
								class="rounded-full px-2 py-0.5 text-[10px]"
								style="background: rgba(79,110,247,0.12); color: #4f6ef7; border: 1px solid rgba(79,110,247,0.25)"
								>{branch.branch}</span
							>
						</Table.Cell>
						<Table.Cell class="font-mono text-sm">{branch.commits_count}</Table.Cell>
						<Table.Cell class="font-mono text-sm">{branch.sessions_count}</Table.Cell>
						<Table.Cell class="font-mono text-sm">{fmtCost(branch.total_cost)}</Table.Cell>
						<Table.Cell>
							<span
								class="rounded-full px-2 py-0.5 text-[10px]"
								style="background: {ss.bg}; color: {ss.color}; border: 1px solid {ss.border}"
								>{branch.status}</span
							>
						</Table.Cell>
						<Table.Cell class="text-muted-foreground"
							>{fmtRelativeTime(branch.last_activity)}</Table.Cell
						>
					</Table.Row>
				{/each}
			</Table.Body>
		</Table.Root>
	{/if}
</div>
