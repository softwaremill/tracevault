<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import * as Card from '$lib/components/ui/card/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';

	interface TraceDetail {
		id: string;
		commit_sha: string;
		parent_sha: string | null;
		author: string | null;
		committer: string | null;
		message: string | null;
		model: string | null;
		ai_percentage: number | null;
		repo_name: string | null;
		session_data: Record<string, unknown> | null;
		attribution: Record<string, unknown> | null;
		created_at: string;
	}

	let trace: TraceDetail | null = $state(null);
	let loading = $state(true);
	let error = $state('');

	const traceId = $derived($page.params.id ?? '');

	onMount(async () => {
		try {
			trace = await api.get<TraceDetail>(`/api/v1/traces/${traceId}`);
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load trace';
		} finally {
			loading = false;
		}
	});

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleString();
	}

	function formatPercentage(val: number | null): string {
		if (val == null) return '-';
		return `${(val * 100).toFixed(1)}%`;
	}

	function formatJson(obj: unknown): string {
		return JSON.stringify(obj, null, 2);
	}
</script>

<svelte:head>
	<title>Trace Detail - TraceVault</title>
</svelte:head>

<div class="space-y-4">
	<div class="flex items-center gap-2">
		<a href="/traces" class="text-muted-foreground hover:underline">Traces</a>
		<span class="text-muted-foreground">/</span>
		<h1 class="text-2xl font-bold font-mono">{traceId.slice(0, 8)}</h1>
	</div>

	{#if loading}
		<p class="text-muted-foreground">Loading...</p>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if trace}
		<div class="grid gap-4 md:grid-cols-2">
			<Card.Root>
				<Card.Header>
					<Card.Title>Commit Info</Card.Title>
				</Card.Header>
				<Card.Content class="space-y-2">
					<div class="flex justify-between">
						<span class="text-muted-foreground">SHA</span>
						<span class="font-mono text-sm">{trace.commit_sha}</span>
					</div>
					{#if trace.parent_sha}
						<div class="flex justify-between">
							<span class="text-muted-foreground">Parent</span>
							<span class="font-mono text-sm">{trace.parent_sha}</span>
						</div>
					{/if}
					<div class="flex justify-between">
						<span class="text-muted-foreground">Author</span>
						<span>{trace.author ?? '-'}</span>
					</div>
					{#if trace.committer}
						<div class="flex justify-between">
							<span class="text-muted-foreground">Committer</span>
							<span>{trace.committer}</span>
						</div>
					{/if}
					{#if trace.message}
						<div>
							<span class="text-muted-foreground">Message</span>
							<p class="mt-1 text-sm">{trace.message}</p>
						</div>
					{/if}
					<div class="flex justify-between">
						<span class="text-muted-foreground">Date</span>
						<span>{formatDate(trace.created_at)}</span>
					</div>
				</Card.Content>
			</Card.Root>

			<Card.Root>
				<Card.Header>
					<Card.Title>AI Attribution</Card.Title>
				</Card.Header>
				<Card.Content class="space-y-2">
					<div class="flex justify-between">
						<span class="text-muted-foreground">Model</span>
						{#if trace.model}
							<Badge>{trace.model}</Badge>
						{:else}
							<span class="text-muted-foreground">-</span>
						{/if}
					</div>
					<div class="flex justify-between">
						<span class="text-muted-foreground">AI Percentage</span>
						<span class="font-semibold">{formatPercentage(trace.ai_percentage)}</span>
					</div>
				</Card.Content>
			</Card.Root>
		</div>

		{#if trace.session_data}
			<Card.Root>
				<Card.Header>
					<Card.Title>Session Data</Card.Title>
				</Card.Header>
				<Card.Content>
					<pre class="overflow-auto rounded bg-muted p-4 text-sm">{formatJson(trace.session_data)}</pre>
				</Card.Content>
			</Card.Root>
		{/if}

		{#if trace.attribution}
			<Card.Root>
				<Card.Header>
					<Card.Title>Attribution Details</Card.Title>
				</Card.Header>
				<Card.Content>
					<pre class="overflow-auto rounded bg-muted p-4 text-sm">{formatJson(trace.attribution)}</pre>
				</Card.Content>
			</Card.Root>
		{/if}
	{/if}
</div>
