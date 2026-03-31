<script lang="ts">
	import { page } from '$app/stores';
	import { useFetch } from '$lib/hooks/use-fetch.svelte';
	import { fmtRelativeTime } from '$lib/utils/format';
	import DataTable from '$lib/components/DataTable.svelte';
	import LoadingState from '$lib/components/LoadingState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import EmptyState from '$lib/components/EmptyState.svelte';

	interface CommitListItem {
		id: string;
		commit_sha: string;
		branch: string | null;
		author: string;
		message: string | null;
		files_changed: number;
		ai_sessions_count: number;
		committed_at: string;
	}

	let expandedId: string | null = $state(null);

	function firstLine(msg: string | null): string {
		if (!msg) return '-';
		return msg.split('\n')[0];
	}

	function toggleExpand(id: string) {
		expandedId = expandedId === id ? null : id;
	}

	const slug = $derived($page.params.slug);

	const commitsUrl = $derived.by(() => {
		const params = new URLSearchParams();
		const repoId = $page.url.searchParams.get('repo_id');
		const branch = $page.url.searchParams.get('branch');
		const from = $page.url.searchParams.get('from');
		const to = $page.url.searchParams.get('to');
		if (repoId) params.set('repo_id', repoId);
		if (branch) params.set('branch', branch);
		if (from) params.set('from', from);
		if (to) params.set('to', to);
		const qs = params.toString();
		return `/api/v1/orgs/${slug}/traces/commits${qs ? '?' + qs : ''}`;
	});

	const commitsQuery = useFetch<CommitListItem[]>(() => commitsUrl, { initial: [] });

	const columns = [
		{ key: 'commit_sha', label: 'Commit' },
		{ key: 'message', label: 'Message' },
		{ key: 'branch', label: 'Branch' },
		{ key: 'author', label: 'Author', sortable: true },
		{ key: 'files_changed', label: 'Files Changed', sortable: true },
		{ key: 'ai_sessions_count', label: 'AI Sessions', sortable: true },
		{ key: 'committed_at', label: 'Committed', sortable: true }
	];

	const commits = $derived(commitsQuery.data ?? []);
	const tableRows = $derived(
		commits.map((c) => ({ ...c }) as Record<string, unknown>)
	);
</script>

<svelte:head>
	<title>Commits - TraceVault</title>
</svelte:head>

<div class="space-y-4">
	{#if commitsQuery.loading}
		<LoadingState />
	{:else if commitsQuery.error}
		<ErrorState message={commitsQuery.error} onRetry={commitsQuery.refetch} />
	{:else if commits.length === 0}
		<EmptyState message="No commits yet." />
	{:else}
		<DataTable
			{columns}
			rows={tableRows}
			searchKeys={['commit_sha', 'author', 'message', 'branch']}
			defaultSort="committed_at"
			rowIdKey="id"
			onRowClick={(row) => toggleExpand(row.id as string)}
			expandedRowId={expandedId}
		>
			{#snippet children({ row, col })}
				{#if col.key === 'commit_sha'}
					<a
						href="/orgs/{slug}/traces/commits/{row.id}"
						class="font-mono text-sm underline"
						onclick={(e) => e.stopPropagation()}
					>
						{String(row.commit_sha).slice(0, 8)}
					</a>
				{:else if col.key === 'message'}
					<span class="max-w-xs truncate text-muted-foreground">{firstLine(row.message as string | null)}</span>
				{:else if col.key === 'branch'}
					{#if row.branch}
						<span
							class="rounded-full px-2 py-0.5 text-[10px]"
							style="background: rgba(79,110,247,0.12); color: #4f6ef7; border: 1px solid rgba(79,110,247,0.25)"
						>{row.branch}</span>
					{:else}
						<span class="text-muted-foreground">-</span>
					{/if}
				{:else if col.key === 'files_changed' || col.key === 'ai_sessions_count'}
					<span class="font-mono text-sm">{row[col.key]}</span>
				{:else if col.key === 'committed_at'}
					<span class="text-muted-foreground">{fmtRelativeTime(row.committed_at as string | null)}</span>
				{:else if col.key === 'author'}
					<span class="text-muted-foreground">{row.author}</span>
				{:else}
					{row[col.key] ?? '-'}
				{/if}
			{/snippet}
			{#snippet expandedRow({ row })}
				{#if row.message}
					<div class="bg-muted/20 py-3 px-4">
						<pre class="whitespace-pre-wrap text-xs text-muted-foreground font-mono">{String(row.message).trim()}</pre>
					</div>
				{/if}
			{/snippet}
		</DataTable>
	{/if}
</div>
