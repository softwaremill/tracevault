<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import * as Card from '$lib/components/ui/card/index.js';
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
	let repoName = $state('');
	let loading = $state(true);
	let error = $state('');

	const repoId = $derived($page.params.id);

	onMount(async () => {
		try {
			const allCommits = await api.get<CommitListItem[]>('/api/v1/traces');
			commits = allCommits;
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
	<title>Repo Detail - TraceVault</title>
</svelte:head>

<div class="space-y-4">
	<div class="flex items-center gap-2">
		<a href="/repos" class="text-muted-foreground hover:underline">Repos</a>
		<span class="text-muted-foreground">/</span>
		<h1 class="text-2xl font-bold">{repoName || repoId}</h1>
	</div>

	<Card.Root>
		<Card.Header>
			<Card.Title>Commits</Card.Title>
		</Card.Header>
		<Card.Content>
			{#if loading}
				<p class="text-muted-foreground">Loading...</p>
			{:else if error}
				<p class="text-destructive">{error}</p>
			{:else if commits.length === 0}
				<p class="text-muted-foreground">No commits found for this repo.</p>
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
									<a href="/traces/{commit.id}" class="font-mono text-sm underline">
										{commit.commit_sha.slice(0, 8)}
									</a>
								</Table.Cell>
								<Table.Cell>{commit.author}</Table.Cell>
								<Table.Cell>
									{#if commit.branch}
										<Badge variant="outline">{commit.branch}</Badge>
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
		</Card.Content>
	</Card.Root>
</div>
