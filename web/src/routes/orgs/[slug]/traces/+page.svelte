<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import * as Table from '$lib/components/ui/table/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';

	interface CommitListItem {
		id: string;
		repo_id: string;
		commit_sha: string;
		branch: string | null;
		author: string;
		session_count: number;
		total_tokens: number | null;
		created_at: string;
	}

	let commits: CommitListItem[] = $state([]);
	let loading = $state(true);
	let error = $state('');

	const slug = $derived($page.params.slug);

	onMount(async () => {
		try {
			commits = await api.get<CommitListItem[]>(`/api/v1/orgs/${$page.params.slug}/traces`);
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load commits';
		} finally {
			loading = false;
		}
	});

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleDateString();
	}

	function fmtTokens(n: number | null): string {
		if (n == null || n === 0) return '-';
		if (n >= 1000) return `${(n / 1000).toFixed(1)}k`;
		return String(n);
	}
</script>

<svelte:head>
	<title>Commits - TraceVault</title>
</svelte:head>

<div class="space-y-4">
	<h1 class="text-2xl font-bold">Commits</h1>

	{#if loading}
		<p class="text-muted-foreground">Loading...</p>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if commits.length === 0}
		<p class="text-muted-foreground">No commits yet. Push traces using the CLI.</p>
	{:else}
		<Table.Root>
			<Table.Header>
				<Table.Row>
					<Table.Head>Commit</Table.Head>
					<Table.Head>Author</Table.Head>
					<Table.Head>Branch</Table.Head>
					<Table.Head>Sessions</Table.Head>
					<Table.Head>Tokens</Table.Head>
					<Table.Head>Date</Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each commits as commit}
					<Table.Row>
						<Table.Cell>
							<a href="/orgs/{slug}/traces/{commit.id}" class="font-mono text-sm underline">
								{commit.commit_sha.slice(0, 8)}
							</a>
						</Table.Cell>
						<Table.Cell><Badge variant="author">{commit.author}</Badge></Table.Cell>
						<Table.Cell>
							{#if commit.branch}
								<Badge variant="branch">{commit.branch}</Badge>
							{:else}
								<span class="text-muted-foreground">-</span>
							{/if}
						</Table.Cell>
						<Table.Cell>{commit.session_count}</Table.Cell>
						<Table.Cell class="font-mono text-sm">{fmtTokens(commit.total_tokens)}</Table.Cell>
						<Table.Cell>{formatDate(commit.created_at)}</Table.Cell>
					</Table.Row>
				{/each}
			</Table.Body>
		</Table.Root>
	{/if}
</div>
