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
		transcript: unknown[] | null;
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

	type ConversationItem =
		| { kind: 'user'; text: string }
		| { kind: 'assistant'; text: string; model: string | null; usage: { input: number; output: number } | null }
		| { kind: 'tool_call'; name: string; input: unknown }
		| { kind: 'tool_result'; content: string };

	function processTranscript(raw: unknown[]): ConversationItem[] {
		const items: ConversationItem[] = [];
		const seenAssistantIds = new Set<string>();

		for (const entry of raw) {
			const e = entry as Record<string, unknown>;
			const type = e.type as string | undefined;
			if (!type || type === 'progress' || type === 'file-history-snapshot') continue;

			const msg = e.message as Record<string, unknown> | undefined;
			if (!msg) continue;

			const content = msg.content;

			if (type === 'user') {
				if (typeof content === 'string') {
					items.push({ kind: 'user', text: content });
				} else if (Array.isArray(content)) {
					for (const block of content) {
						const b = block as Record<string, unknown>;
						if (b.type === 'tool_result') {
							const resultContent = b.content;
							let text: string;
							if (typeof resultContent === 'string') {
								text = resultContent;
							} else if (Array.isArray(resultContent)) {
								text = resultContent
									.filter((c: Record<string, unknown>) => c.type === 'text')
									.map((c: Record<string, unknown>) => c.text as string)
									.join('\n');
							} else {
								text = JSON.stringify(resultContent, null, 2);
							}
							if (text) {
								items.push({ kind: 'tool_result', content: text });
							}
						}
					}
				}
			} else if (type === 'assistant') {
				const msgId = msg.id as string | undefined;
				if (msgId && seenAssistantIds.has(msgId)) continue;
				if (msgId) seenAssistantIds.add(msgId);

				if (Array.isArray(content)) {
					const model = (msg.model as string) ?? null;
					const rawUsage = msg.usage as Record<string, number> | undefined;
					const usage = rawUsage
						? { input: rawUsage.input_tokens ?? 0, output: rawUsage.output_tokens ?? 0 }
						: null;

					const textParts: string[] = [];
					for (const block of content) {
						const b = block as Record<string, unknown>;
						if (b.type === 'thinking') continue;
						if (b.type === 'text') {
							textParts.push(b.text as string);
						} else if (b.type === 'tool_use') {
							items.push({ kind: 'tool_call', name: b.name as string, input: b.input });
						}
					}
					if (textParts.length > 0) {
						items.push({ kind: 'assistant', text: textParts.join('\n'), model, usage });
					}
				}
			}
		}
		return items;
	}

	const conversationItems = $derived.by(() => {
		if (!trace || !trace.transcript) return [] as ConversationItem[];
		return processTranscript(trace.transcript);
	});
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

		{#if trace.transcript && conversationItems.length > 0}
			<Card.Root>
				<Card.Header>
					<Card.Title class="flex items-center gap-2">
						Transcript
						<Badge variant="secondary">{conversationItems.length} events</Badge>
					</Card.Title>
				</Card.Header>
				<Card.Content>
					<div class="space-y-3">
						{#each conversationItems as item}
							{#if item.kind === 'user'}
								<div class="border-l-4 border-sky-300 pl-4 py-2">
									<div class="text-xs font-semibold text-sky-500 dark:text-sky-300 mb-1">User</div>
									<p class="text-sm whitespace-pre-wrap">{item.text}</p>
								</div>
							{:else if item.kind === 'assistant'}
								<div class="border-l-4 border-green-500 pl-4 py-2">
									<div class="flex items-center gap-2 mb-1">
										<span class="text-xs font-semibold text-green-600 dark:text-green-400">Assistant</span>
										{#if item.model}
											<Badge variant="outline" class="text-xs">{item.model}</Badge>
										{/if}
										{#if item.usage}
											<span class="text-xs text-muted-foreground">
												{item.usage.input + item.usage.output} tokens
											</span>
										{/if}
									</div>
									<p class="text-sm whitespace-pre-wrap">{item.text}</p>
								</div>
							{:else if item.kind === 'tool_call'}
								<div class="border-l-4 border-amber-500 pl-4 py-2">
									<details>
										<summary class="cursor-pointer text-xs font-semibold text-amber-600 dark:text-amber-400">
											Tool: {item.name}
										</summary>
										<pre class="mt-2 overflow-auto rounded bg-muted p-3 font-mono text-xs max-h-80">{formatJson(item.input)}</pre>
									</details>
								</div>
							{:else if item.kind === 'tool_result'}
								<div class="border-l-4 border-gray-400 pl-4 py-2">
									<details>
										<summary class="cursor-pointer text-xs font-semibold text-muted-foreground">
											Tool Result
										</summary>
										<pre class="mt-2 overflow-auto rounded bg-muted p-3 font-mono text-xs max-h-80">{item.content}</pre>
									</details>
								</div>
							{/if}
						{/each}
					</div>
				</Card.Content>
			</Card.Root>
		{/if}
	{/if}
</div>
