<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import * as Table from '$lib/components/ui/table/index.js';

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
		<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm">
			<span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"></span>
			Loading...
		</div>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if repos.length === 0}
		<p class="text-muted-foreground">No repos registered yet. Use the CLI to add one.</p>
	{:else}
		<Table.Root>
			<Table.Header>
				<Table.Row>
					<Table.Head class="text-xs">Name</Table.Head>
					<Table.Head class="text-xs">GitHub URL</Table.Head>
					<Table.Head class="text-xs">Created</Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each repos as repo}
					<Table.Row class="hover:bg-muted/40 transition-colors">
						<Table.Cell class="text-xs">
							<a href="/orgs/{slug}/repos/{repo.id}" class="font-medium underline">{repo.name}</a>
						</Table.Cell>
						<Table.Cell class="text-xs">
							{#if repo.github_url}
								<span class="rounded-full px-2 py-0.5 text-[10px]" style="background: rgba(167,139,250,0.12); color: #a78bfa; border: 1px solid rgba(167,139,250,0.25)">{repo.github_url}</span>
							{:else}
								<span class="text-muted-foreground">-</span>
							{/if}
						</Table.Cell>
						<Table.Cell class="text-xs">{formatDate(repo.created_at)}</Table.Cell>
					</Table.Row>
				{/each}
			</Table.Body>
		</Table.Root>
	{/if}
</div>
