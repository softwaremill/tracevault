<script lang="ts">
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import * as Table from '$lib/components/ui/table/index.js';

	interface SessionItem {
		id: string;
		session_id: string;
		repo_id: string;
		repo_name: string;
		user_id: string | null;
		user_email: string | null;
		status: string;
		model: string | null;
		tool: string | null;
		total_tool_calls: number | null;
		total_tokens: number | null;
		estimated_cost_usd: number | null;
		cwd: string | null;
		started_at: string | null;
		updated_at: string | null;
	}

	let sessions: SessionItem[] = $state([]);
	let loading = $state(true);
	let error = $state('');
	let statusFilter = $state<'all' | 'active' | 'completed' | 'stale'>('all');

	const slug = $derived($page.params.slug);

	function displayStatus(session: SessionItem): 'active' | 'completed' | 'stale' {
		if (session.status === 'completed') return 'completed';
		if (session.status === 'active' && session.updated_at) {
			const updatedAt = new Date(session.updated_at).getTime();
			const thirtyMinAgo = Date.now() - 30 * 60 * 1000;
			if (updatedAt < thirtyMinAgo) return 'stale';
		}
		return 'active';
	}

	const statusColors: Record<string, { bg: string; text: string; label: string }> = {
		active: { bg: 'bg-green-500/15', text: 'text-green-600 dark:text-green-400', label: 'Active' },
		completed: { bg: 'bg-zinc-500/15', text: 'text-zinc-500 dark:text-zinc-400', label: 'Completed' },
		stale: { bg: 'bg-yellow-500/15', text: 'text-yellow-600 dark:text-yellow-400', label: 'Stale' }
	};

	async function fetchSessions() {
		loading = true;
		error = '';
		try {
			const params = new URLSearchParams();
			const repoId = $page.url.searchParams.get('repo_id');
			const from = $page.url.searchParams.get('from');
			const to = $page.url.searchParams.get('to');
			if (repoId) params.set('repo_id', repoId);
			if (from) params.set('from', from);
			if (to) params.set('to', to);
			if (statusFilter !== 'all') params.set('status', statusFilter);
			const qs = params.toString();
			sessions = await api.get<SessionItem[]>(
				`/api/v1/orgs/${slug}/traces/sessions${qs ? '?' + qs : ''}`
			);
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load sessions';
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void slug;
		void $page.url.search;
		void statusFilter;
		fetchSessions();
	});

	function fmtNum(n: number | null): string {
		if (n == null) return '-';
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

	const filterButtons: { value: typeof statusFilter; label: string }[] = [
		{ value: 'all', label: 'All' },
		{ value: 'active', label: 'Active' },
		{ value: 'completed', label: 'Completed' },
		{ value: 'stale', label: 'Stale' }
	];
</script>

<svelte:head>
	<title>Sessions - TraceVault</title>
</svelte:head>

<div class="space-y-4">
	<!-- Status filter -->
	<div class="flex gap-1">
		{#each filterButtons as btn}
			<button
				class="rounded-md px-3 py-1.5 text-xs font-medium transition-colors
					{statusFilter === btn.value
						? 'bg-primary text-primary-foreground'
						: 'bg-muted text-muted-foreground hover:text-foreground'}"
				onclick={() => (statusFilter = btn.value)}
			>
				{btn.label}
			</button>
		{/each}
	</div>

	{#if loading}
		<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm">
			<span
				class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
			></span>
			Loading sessions...
		</div>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if sessions.length === 0}
		<p class="text-muted-foreground py-8 text-center text-sm">No sessions found.</p>
	{:else}
		<div class="border-border overflow-hidden rounded-lg border">
			<Table.Root class="text-xs">
				<Table.Header>
					<Table.Row>
						<Table.Head>Status</Table.Head>
						<Table.Head>Session ID</Table.Head>
						<Table.Head>Repo</Table.Head>
						<Table.Head>Tool Calls</Table.Head>
						<Table.Head>Tokens</Table.Head>
						<Table.Head>Started</Table.Head>
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each sessions as session (session.id)}
						{@const status = displayStatus(session)}
						{@const sc = statusColors[status]}
						<Table.Row class="hover:bg-muted/40 transition-colors">
							<Table.Cell>
								<span class="inline-flex items-center rounded-full px-2 py-0.5 text-[10px] font-medium {sc.bg} {sc.text}">
									{sc.label}
								</span>
							</Table.Cell>
							<Table.Cell>
								<a
									href="/orgs/{slug}/traces/sessions/{session.id}"
									class="font-mono text-sm underline"
								>
									{session.session_id.slice(0, 8)}
								</a>
							</Table.Cell>
							<Table.Cell>{session.repo_name}</Table.Cell>
							<Table.Cell class="font-mono text-sm">{fmtNum(session.total_tool_calls)}</Table.Cell>
							<Table.Cell class="font-mono text-sm">{fmtNum(session.total_tokens)}</Table.Cell>
							<Table.Cell class="text-sm">{fmtRelativeTime(session.started_at)}</Table.Cell>
						</Table.Row>
					{/each}
				</Table.Body>
			</Table.Root>
		</div>
	{/if}
</div>
