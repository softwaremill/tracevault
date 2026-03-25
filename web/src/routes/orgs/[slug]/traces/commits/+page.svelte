<script lang="ts">
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import * as Table from '$lib/components/ui/table/index.js';

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

	let commits: CommitListItem[] = $state([]);
	let loading = $state(true);
	let error = $state('');
	let expandedId: string | null = $state(null);

	function firstLine(msg: string | null): string {
		if (!msg) return '-';
		return msg.split('\n')[0];
	}

	function toggleExpand(id: string) {
		expandedId = expandedId === id ? null : id;
	}

	const slug = $derived($page.params.slug);

	async function fetchData(search: string) {
		loading = true;
		error = '';
		try {
			commits = await api.get<CommitListItem[]>(
				`/api/v1/orgs/${slug}/traces/commits` + (search ? '?' + search : '')
			);
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load commits';
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		const search = $page.url.search.replace(/^\?/, '');
		fetchData(search);
	});

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
</script>

<svelte:head>
	<title>Commits - TraceVault</title>
</svelte:head>

<div class="space-y-4">
	<h1 class="text-2xl font-bold">Commits</h1>

	{#if loading}
		<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm">
			<span
				class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
			></span>
			Loading...
		</div>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if commits.length === 0}
		<p class="text-muted-foreground">No commits yet.</p>
	{:else}
		<Table.Root class="text-xs">
			<Table.Header>
				<Table.Row>
					<Table.Head>Commit</Table.Head>
					<Table.Head>Message</Table.Head>
					<Table.Head>Branch</Table.Head>
					<Table.Head>Author</Table.Head>
					<Table.Head>Files Changed</Table.Head>
					<Table.Head>AI Sessions</Table.Head>
					<Table.Head>Committed</Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each commits as commit}
					<Table.Row
						class="hover:bg-muted/40 cursor-pointer transition-colors"
						onclick={() => toggleExpand(commit.id)}
					>
						<Table.Cell>
							<a
								href="/orgs/{slug}/traces/commits/{commit.id}"
								class="font-mono text-sm underline"
								onclick={(e) => e.stopPropagation()}
							>
								{commit.commit_sha.slice(0, 8)}
							</a>
						</Table.Cell>
						<Table.Cell class="max-w-xs truncate text-muted-foreground"
							>{firstLine(commit.message)}</Table.Cell
						>
						<Table.Cell>
							{#if commit.branch}
								<span
									class="rounded-full px-2 py-0.5 text-[10px]"
									style="background: rgba(79,110,247,0.12); color: #4f6ef7; border: 1px solid rgba(79,110,247,0.25)"
									>{commit.branch}</span
								>
							{:else}
								<span class="text-muted-foreground">-</span>
							{/if}
						</Table.Cell>
						<Table.Cell class="text-muted-foreground">{commit.author}</Table.Cell>
						<Table.Cell class="font-mono text-sm">{commit.files_changed}</Table.Cell>
						<Table.Cell class="font-mono text-sm">{commit.ai_sessions_count}</Table.Cell>
						<Table.Cell class="text-muted-foreground"
							>{fmtRelativeTime(commit.committed_at)}</Table.Cell
						>
					</Table.Row>
					{#if expandedId === commit.id && commit.message}
						<Table.Row class="bg-muted/20">
							<Table.Cell colspan={7} class="py-3 px-4">
								<pre class="whitespace-pre-wrap text-xs text-muted-foreground font-mono">{commit.message.trim()}</pre>
							</Table.Cell>
						</Table.Row>
					{/if}
				{/each}
			</Table.Body>
		</Table.Root>
	{/if}
</div>
