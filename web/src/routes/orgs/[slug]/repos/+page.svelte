<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import * as Table from '$lib/components/ui/table/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';

	interface Repo {
		id: string;
		name: string;
		github_url: string | null;
		created_at: string;
	}

	let repos: Repo[] = $state([]);
	let loading = $state(true);
	let error = $state('');

	const slug = $derived($page.params.slug);

	onMount(async () => {
		try {
			repos = await api.get<Repo[]>(`/api/v1/orgs/${$page.params.slug}/repos`);
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load repos';
		} finally {
			loading = false;
		}
	});

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleDateString();
	}
</script>

<svelte:head>
	<title>Repos - TraceVault</title>
</svelte:head>

<div class="space-y-4">
	<h1 class="text-2xl font-bold">Repos</h1>

	{#if loading}
		<p class="text-muted-foreground">Loading...</p>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if repos.length === 0}
		<p class="text-muted-foreground">No repos registered yet. Use the CLI to add one.</p>
	{:else}
		<Table.Root>
			<Table.Header>
				<Table.Row>
					<Table.Head>Name</Table.Head>
					<Table.Head>GitHub URL</Table.Head>
					<Table.Head>Created</Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each repos as repo}
					<Table.Row>
						<Table.Cell>
							<a href="/orgs/{slug}/repos/{repo.id}" class="font-medium underline">{repo.name}</a>
						</Table.Cell>
						<Table.Cell>
							{#if repo.github_url}
								<Badge variant="branch">{repo.github_url}</Badge>
							{:else}
								<span class="text-muted-foreground">-</span>
							{/if}
						</Table.Cell>
						<Table.Cell>{formatDate(repo.created_at)}</Table.Cell>
					</Table.Row>
				{/each}
			</Table.Body>
		</Table.Root>
	{/if}
</div>
