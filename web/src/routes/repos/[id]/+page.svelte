<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Table from '$lib/components/ui/table/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';

	interface Trace {
		id: string;
		commit_sha: string;
		author: string | null;
		model: string | null;
		ai_percentage: number | null;
		created_at: string;
	}

	let traces: Trace[] = $state([]);
	let repoName = $state('');
	let loading = $state(true);
	let error = $state('');

	const repoId = $derived($page.params.id);

	onMount(async () => {
		try {
			// Fetch traces for this repo - the API may filter by repo_id or we load all and filter
			const allTraces = await api.get<Trace[]>('/api/v1/traces');
			traces = allTraces;
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load traces';
		} finally {
			loading = false;
		}
	});

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleDateString();
	}

	function formatPercentage(val: number | null): string {
		if (val == null) return '-';
		return `${(val * 100).toFixed(1)}%`;
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
			<Card.Title>Traces</Card.Title>
		</Card.Header>
		<Card.Content>
			{#if loading}
				<p class="text-muted-foreground">Loading...</p>
			{:else if error}
				<p class="text-destructive">{error}</p>
			{:else if traces.length === 0}
				<p class="text-muted-foreground">No traces found for this repo.</p>
			{:else}
				<Table.Root>
					<Table.Header>
						<Table.Row>
							<Table.Head>Commit</Table.Head>
							<Table.Head>Author</Table.Head>
							<Table.Head>Model</Table.Head>
							<Table.Head>AI %</Table.Head>
							<Table.Head>Date</Table.Head>
						</Table.Row>
					</Table.Header>
					<Table.Body>
						{#each traces as trace}
							<Table.Row>
								<Table.Cell>
									<a href="/traces/{trace.id}" class="font-mono text-sm underline">
										{trace.commit_sha.slice(0, 8)}
									</a>
								</Table.Cell>
								<Table.Cell>{trace.author ?? '-'}</Table.Cell>
								<Table.Cell>
									{#if trace.model}
										<Badge variant="outline">{trace.model}</Badge>
									{:else}
										<span class="text-muted-foreground">-</span>
									{/if}
								</Table.Cell>
								<Table.Cell>{formatPercentage(trace.ai_percentage)}</Table.Cell>
								<Table.Cell>{formatDate(trace.created_at)}</Table.Cell>
							</Table.Row>
						{/each}
					</Table.Body>
				</Table.Root>
			{/if}
		</Card.Content>
	</Card.Root>
</div>
