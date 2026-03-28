<script lang="ts">
	import { page } from '$app/stores';
	import { useFetch } from '$lib/hooks/use-fetch.svelte';
	import { fmtNum, fmtRelativeTime } from '$lib/utils/format';
	import { sessionStatus } from '$lib/utils/status';
	import type { SessionItem } from '$lib/types';
	import DataTable from '$lib/components/DataTable.svelte';
	import StatusBadge from '$lib/components/StatusBadge.svelte';
	import LoadingState from '$lib/components/LoadingState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';

	let statusFilter = $state<'all' | 'active' | 'completed' | 'stale'>('all');

	const slug = $derived($page.params.slug);

	const sessionsUrl = $derived.by(() => {
		const params = new URLSearchParams();
		const repoId = $page.url.searchParams.get('repo_id');
		const from = $page.url.searchParams.get('from');
		const to = $page.url.searchParams.get('to');
		if (repoId) params.set('repo_id', repoId);
		if (from) params.set('from', from);
		if (to) params.set('to', to);
		if (statusFilter !== 'all') params.set('status', statusFilter);
		const qs = params.toString();
		return `/api/v1/orgs/${slug}/traces/sessions${qs ? '?' + qs : ''}`;
	});

	const sessionsQuery = useFetch<SessionItem[]>(() => sessionsUrl, { initial: [] });

	const filterButtons: { value: typeof statusFilter; label: string }[] = [
		{ value: 'all', label: 'All' },
		{ value: 'active', label: 'Active' },
		{ value: 'completed', label: 'Completed' },
		{ value: 'stale', label: 'Stale' }
	];

	const columns = [
		{ key: '_status', label: 'Status' },
		{ key: 'session_id', label: 'Session ID' },
		{ key: 'repo_name', label: 'Repo', sortable: true },
		{ key: 'total_tool_calls', label: 'Tool Calls', sortable: true },
		{ key: 'total_tokens', label: 'Tokens', sortable: true },
		{ key: 'started_at', label: 'Started', sortable: true }
	];

	const sessions = $derived(sessionsQuery.data ?? []);
	const tableRows = $derived(
		sessions.map((s) => ({ ...s, _status: sessionStatus(s.status, s.updated_at) }) as Record<string, unknown>)
	);
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

	{#if sessionsQuery.loading}
		<LoadingState />
	{:else if sessionsQuery.error}
		<ErrorState message={sessionsQuery.error} onRetry={sessionsQuery.refetch} />
	{:else if sessions.length === 0}
		<EmptyState message="No sessions found." />
	{:else}
		<DataTable
			{columns}
			rows={tableRows}
			searchKeys={['session_id', 'repo_name']}
			defaultSort="started_at"
			rowIdKey="id"
		>
			{#snippet children({ row, col })}
				{#if col.key === '_status'}
					<StatusBadge status={String(row._status)} />
				{:else if col.key === 'session_id'}
					<a href="/orgs/{slug}/traces/sessions/{row.id}" class="font-mono text-sm underline">
						{String(row.session_id).slice(0, 8)}
					</a>
				{:else if col.key === 'total_tool_calls'}
					<span class="font-mono text-sm">{fmtNum(row.total_tool_calls as number | null)}</span>
				{:else if col.key === 'total_tokens'}
					<span class="font-mono text-sm">{fmtNum(row.total_tokens as number | null)}</span>
				{:else if col.key === 'started_at'}
					{fmtRelativeTime(row.started_at as string | null)}
				{:else}
					{row[col.key] ?? '-'}
				{/if}
			{/snippet}
		</DataTable>
	{/if}
</div>
