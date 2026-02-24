<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Table from '$lib/components/ui/table/index.js';
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
		diff_data: FileDiff[] | null;
		created_at: string;
	}

	interface TokenUsage {
		input_tokens: number;
		output_tokens: number;
		cache_creation_input_tokens: number;
		cache_read_input_tokens: number;
	}

	interface TranscriptEntry {
		index: number;
		timestamp: string | null;
		type: string;
		subtype: string | null;
		summary: string;
		model: string | null;
		usage: TokenUsage | null;
		prompt: string | null;
		toolNames: string[];
		raw: unknown;
	}

	interface ModelStats {
		model: string;
		tokens: number;
		count: number;
	}

	interface TranscriptStats {
		totalInputTokens: number;
		totalOutputTokens: number;
		totalCacheReadTokens: number;
		totalCacheCreationTokens: number;
		byModel: ModelStats[];
		toolUsageCounts: Record<string, number>;
		turnCount: number;
		userMessageCount: number;
		totalDurationMs: number;
	}

	interface DiffLine {
		kind: 'add' | 'delete' | 'context';
		content: string;
		new_line_number: number | null;
		old_line_number: number | null;
	}

	interface DiffHunk {
		old_start: number;
		old_count: number;
		new_start: number;
		new_count: number;
		lines: DiffLine[];
	}

	interface FileDiff {
		path: string;
		old_path: string | null;
		hunks: DiffHunk[];
	}

	interface AttrLineRange {
		start: number;
		end: number;
	}

	interface FileAttribution {
		path: string;
		lines_added: number;
		lines_deleted: number;
		ai_lines: AttrLineRange[];
		human_lines: AttrLineRange[];
		mixed_lines: AttrLineRange[];
	}

	interface AttributionData {
		files: FileAttribution[];
		summary: {
			total_lines_added: number;
			total_lines_deleted: number;
			ai_percentage: number;
			human_percentage: number;
		};
	}

	function truncate(s: string, max: number): string {
		if (s.length <= max) return s;
		return s.slice(0, max) + '…';
	}

	function parseTranscript(raw: unknown[]): TranscriptEntry[] {
		const entries: TranscriptEntry[] = [];

		for (let i = 0; i < raw.length; i++) {
			const e = raw[i] as Record<string, unknown>;
			const type = (e.type as string) ?? 'unknown';
			const subtype = (e.subtype as string) ?? null;
			const timestamp = (e.timestamp as string) ?? null;
			const msg = e.message as Record<string, unknown> | undefined;

			let summary = type;
			let model: string | null = null;
			let usage: TokenUsage | null = null;
			let prompt: string | null = null;
			const toolNames: string[] = [];

			if (type === 'assistant' && msg) {
				model = (msg.model as string) ?? null;
				const rawUsage = msg.usage as Record<string, number> | undefined;
				if (rawUsage) {
					usage = {
						input_tokens: rawUsage.input_tokens ?? 0,
						output_tokens: rawUsage.output_tokens ?? 0,
						cache_creation_input_tokens: rawUsage.cache_creation_input_tokens ?? 0,
						cache_read_input_tokens: rawUsage.cache_read_input_tokens ?? 0
					};
				}
				const content = msg.content;
				if (Array.isArray(content)) {
					const textParts: string[] = [];
					for (const block of content) {
						const b = block as Record<string, unknown>;
						if (b.type === 'text') textParts.push(b.text as string);
						else if (b.type === 'tool_use') toolNames.push(b.name as string);
					}
					if (textParts.length > 0) {
						summary = truncate(textParts.join(' ').replace(/\s+/g, ' '), 120);
					} else if (toolNames.length > 0) {
						summary = `tool calls: ${toolNames.join(', ')}`;
					} else {
						summary = 'assistant response';
					}
				}
			} else if (type === 'user') {
				const content = msg?.content;
				if (typeof content === 'string') {
					prompt = content;
					summary = truncate(content.replace(/\s+/g, ' '), 120);
				} else if (Array.isArray(content)) {
					const toolResults = content.filter(
						(b: Record<string, unknown>) => (b as Record<string, unknown>).type === 'tool_result'
					);
					summary = `${toolResults.length} tool result${toolResults.length !== 1 ? 's' : ''}`;
				} else {
					summary = 'user message';
				}
			} else if (type === 'progress' && subtype === 'agent_progress') {
				const data = e.data as Record<string, unknown> | undefined;
				if (data) {
					prompt = (data.prompt as string) ?? null;
					const nestedMsg = data.message as Record<string, unknown> | undefined;
					const innerMsg = nestedMsg?.message as Record<string, unknown> | undefined;
					if (innerMsg) {
						model = (innerMsg.model as string) ?? null;
						const rawUsage = innerMsg.usage as Record<string, number> | undefined;
						if (rawUsage) {
							usage = {
								input_tokens: rawUsage.input_tokens ?? 0,
								output_tokens: rawUsage.output_tokens ?? 0,
								cache_creation_input_tokens: rawUsage.cache_creation_input_tokens ?? 0,
								cache_read_input_tokens: rawUsage.cache_read_input_tokens ?? 0
							};
						}
					}
					summary = prompt ? truncate(prompt.replace(/\s+/g, ' '), 120) : 'agent progress';
				}
			} else if (type === 'system' && subtype === 'turn_duration') {
				const data = e.data as Record<string, unknown> | undefined;
				const durationMs = (data?.durationMs as number) ?? 0;
				summary = `turn duration: ${fmtDuration(durationMs)}`;
			}

			entries.push({
				index: i,
				timestamp,
				type,
				subtype: subtype,
				summary,
				model,
				usage,
				prompt,
				toolNames,
				raw: e
			});
		}

		return entries;
	}

	function computeStats(entries: TranscriptEntry[]): TranscriptStats {
		let totalInputTokens = 0;
		let totalOutputTokens = 0;
		let totalCacheReadTokens = 0;
		let totalCacheCreationTokens = 0;
		let userMessageCount = 0;
		let totalDurationMs = 0;
		const modelMap = new Map<string, { tokens: number; count: number }>();
		const toolCounts: Record<string, number> = {};

		for (const entry of entries) {
			if (entry.usage) {
				totalInputTokens += entry.usage.input_tokens;
				totalOutputTokens += entry.usage.output_tokens;
				totalCacheReadTokens += entry.usage.cache_read_input_tokens;
				totalCacheCreationTokens += entry.usage.cache_creation_input_tokens;
			}
			if (entry.model && entry.usage) {
				const existing = modelMap.get(entry.model) ?? { tokens: 0, count: 0 };
				existing.tokens +=
					entry.usage.input_tokens +
					entry.usage.output_tokens +
					entry.usage.cache_read_input_tokens +
					entry.usage.cache_creation_input_tokens;
				existing.count++;
				modelMap.set(entry.model, existing);
			}
			if (entry.type === 'user' && entry.prompt) {
				userMessageCount++;
			}
			for (const tool of entry.toolNames) {
				toolCounts[tool] = (toolCounts[tool] ?? 0) + 1;
			}
			if (entry.type === 'system' && entry.subtype === 'turn_duration') {
				const data = (entry.raw as Record<string, unknown>).data as
					| Record<string, unknown>
					| undefined;
				totalDurationMs += (data?.durationMs as number) ?? 0;
			}
		}

		const byModel: ModelStats[] = Array.from(modelMap.entries())
			.map(([model, { tokens, count }]) => ({ model, tokens, count }))
			.sort((a, b) => b.tokens - a.tokens);

		return {
			totalInputTokens,
			totalOutputTokens,
			totalCacheReadTokens,
			totalCacheCreationTokens,
			byModel,
			toolUsageCounts: toolCounts,
			turnCount: entries.length,
			userMessageCount,
			totalDurationMs
		};
	}

	function fmtTokens(n: number | undefined): string {
		if (n == null || n === 0) return '-';
		if (n >= 1000) return `${(n / 1000).toFixed(1)}k`;
		return String(n);
	}

	function fmtTime(iso: string | null): string {
		if (!iso) return '-';
		return new Date(iso).toLocaleTimeString();
	}

	function fmtDuration(ms: number): string {
		if (ms === 0) return '-';
		if (ms < 1000) return `${Math.round(ms)}ms`;
		return `${(ms / 1000).toFixed(1)}s`;
	}

	let trace: TraceDetail | null = $state(null);
	let loading = $state(true);
	let error = $state('');
	let expandedRows: Set<number> = $state(new Set());
	let visibleTypes: Set<string> = $state(new Set());

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

	const transcriptEntries = $derived.by(() => {
		if (!trace?.transcript) return [] as TranscriptEntry[];
		return parseTranscript(trace.transcript);
	});

	const stats = $derived.by(() => computeStats(transcriptEntries));

	const allTypes = $derived.by(() => {
		const types = new Set<string>();
		for (const entry of transcriptEntries) types.add(entry.type);
		return Array.from(types);
	});

	// Initialize visibleTypes when entries load
	$effect(() => {
		if (transcriptEntries.length > 0 && visibleTypes.size === 0) {
			visibleTypes = new Set(allTypes);
		}
	});

	const filteredEntries = $derived.by(() => {
		if (visibleTypes.size === 0) return transcriptEntries;
		return transcriptEntries.filter((e) => visibleTypes.has(e.type));
	});

	function toggleRow(index: number) {
		const next = new Set(expandedRows);
		if (next.has(index)) next.delete(index);
		else next.add(index);
		expandedRows = next;
	}

	function toggleType(type: string) {
		const next = new Set(visibleTypes);
		if (next.has(type)) next.delete(type);
		else next.add(type);
		visibleTypes = next;
	}

	const sortedToolEntries = $derived.by(() =>
		Object.entries(stats.toolUsageCounts).sort((a, b) => b[1] - a[1])
	);

	let expandedFiles: Set<string> = $state(new Set());

	const diffFiles = $derived.by(() => {
		if (!trace?.diff_data) return [] as FileDiff[];
		return trace.diff_data as FileDiff[];
	});

	const attrData = $derived.by(() => {
		if (!trace?.attribution) return null;
		return trace.attribution as unknown as AttributionData;
	});

	const diffSummary = $derived.by(() => {
		let totalAdded = 0;
		let totalDeleted = 0;
		for (const file of diffFiles) {
			for (const hunk of file.hunks) {
				for (const line of hunk.lines) {
					if (line.kind === 'add') totalAdded++;
					else if (line.kind === 'delete') totalDeleted++;
				}
			}
		}
		return { totalAdded, totalDeleted, fileCount: diffFiles.length };
	});

	function toggleFile(path: string) {
		const next = new Set(expandedFiles);
		if (next.has(path)) next.delete(path);
		else next.add(path);
		expandedFiles = next;
	}

	function isAiLine(filePath: string, lineNum: number): boolean {
		if (!attrData) return false;
		const fileAttr = attrData.files.find((f) => f.path === filePath);
		if (!fileAttr) return false;
		return fileAttr.ai_lines.some((r) => lineNum >= r.start && lineNum <= r.end);
	}

	function fileAiLineCount(filePath: string): number {
		if (!attrData) return 0;
		const fileAttr = attrData.files.find((f) => f.path === filePath);
		if (!fileAttr) return 0;
		return fileAttr.ai_lines.reduce((sum, r) => sum + (r.end - r.start + 1), 0);
	}

	function fileAddedCount(file: FileDiff): number {
		return file.hunks.reduce(
			(sum, h) => sum + h.lines.filter((l) => l.kind === 'add').length,
			0
		);
	}

	function fileDeletedCount(file: FileDiff): number {
		return file.hunks.reduce(
			(sum, h) => sum + h.lines.filter((l) => l.kind === 'delete').length,
			0
		);
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

		{#if trace.transcript && transcriptEntries.length > 0}
			<div class="grid gap-4 md:grid-cols-4">
				<Card.Root>
					<Card.Header class="pb-2">
						<Card.Title class="text-sm font-medium text-muted-foreground">Total Tokens</Card.Title>
					</Card.Header>
					<Card.Content>
						<div class="text-2xl font-bold">{fmtTokens(stats.totalInputTokens + stats.totalOutputTokens)}</div>
						<p class="text-xs text-muted-foreground">
							{fmtTokens(stats.totalInputTokens)} in / {fmtTokens(stats.totalOutputTokens)} out
						</p>
					</Card.Content>
				</Card.Root>

				<Card.Root>
					<Card.Header class="pb-2">
						<Card.Title class="text-sm font-medium text-muted-foreground">Cache Tokens</Card.Title>
					</Card.Header>
					<Card.Content>
						<div class="text-2xl font-bold">{fmtTokens(stats.totalCacheReadTokens + stats.totalCacheCreationTokens)}</div>
						<p class="text-xs text-muted-foreground">
							{fmtTokens(stats.totalCacheReadTokens)} read / {fmtTokens(stats.totalCacheCreationTokens)} created
						</p>
					</Card.Content>
				</Card.Root>

				<Card.Root>
					<Card.Header class="pb-2">
						<Card.Title class="text-sm font-medium text-muted-foreground">Turns</Card.Title>
					</Card.Header>
					<Card.Content>
						<div class="text-2xl font-bold">{stats.userMessageCount}</div>
						<p class="text-xs text-muted-foreground">
							user messages / {stats.turnCount} total events
						</p>
					</Card.Content>
				</Card.Root>

				<Card.Root>
					<Card.Header class="pb-2">
						<Card.Title class="text-sm font-medium text-muted-foreground">Duration</Card.Title>
					</Card.Header>
					<Card.Content>
						<div class="text-2xl font-bold">{fmtDuration(stats.totalDurationMs)}</div>
						<div class="flex flex-wrap gap-1 mt-1">
							{#each stats.byModel as m}
								<Badge variant="outline" class="text-xs">{m.model} ({m.count})</Badge>
							{/each}
						</div>
					</Card.Content>
				</Card.Root>
			</div>

			{#if sortedToolEntries.length > 0}
				<Card.Root>
					<Card.Header class="pb-2">
						<Card.Title class="text-sm font-medium">Tool Usage</Card.Title>
					</Card.Header>
					<Card.Content>
						<div class="flex flex-wrap gap-2">
							{#each sortedToolEntries as [tool, count]}
								<Badge variant="secondary">{tool} ({count})</Badge>
							{/each}
						</div>
					</Card.Content>
				</Card.Root>
			{/if}

			<div class="flex flex-wrap items-center gap-2">
				<span class="text-sm text-muted-foreground">Filter:</span>
				{#each allTypes as type}
					<button onclick={() => toggleType(type)}>
						<Badge variant={visibleTypes.has(type) ? 'default' : 'outline'}>{type}</Badge>
					</button>
				{/each}
			</div>

			<Card.Root>
				<Card.Content class="p-0">
					<Table.Root>
						<Table.Header>
							<Table.Row>
								<Table.Head class="w-12">#</Table.Head>
								<Table.Head class="w-24">Time</Table.Head>
								<Table.Head class="w-28">Type</Table.Head>
								<Table.Head>Summary</Table.Head>
								<Table.Head class="w-36">Model</Table.Head>
								<Table.Head class="w-20 text-right">In</Table.Head>
								<Table.Head class="w-20 text-right">Out</Table.Head>
								<Table.Head class="w-20 text-right">Cache</Table.Head>
							</Table.Row>
						</Table.Header>
						<Table.Body>
							{#each filteredEntries as entry (entry.index)}
								<Table.Row
									class="cursor-pointer hover:bg-muted/50"
									onclick={() => toggleRow(entry.index)}
								>
									<Table.Cell class="font-mono text-xs text-muted-foreground">{entry.index}</Table.Cell>
									<Table.Cell class="text-xs">{fmtTime(entry.timestamp)}</Table.Cell>
									<Table.Cell>
										<Badge variant="outline" class="text-xs">{entry.type}{entry.subtype ? `:${entry.subtype}` : ''}</Badge>
									</Table.Cell>
									<Table.Cell class="max-w-md truncate" title={entry.summary}>
										{entry.summary}
									</Table.Cell>
									<Table.Cell>
										{#if entry.model}
											<Badge variant="secondary" class="text-xs">{entry.model}</Badge>
										{:else}
											<span class="text-muted-foreground">-</span>
										{/if}
									</Table.Cell>
									<Table.Cell class="text-right font-mono text-xs">{fmtTokens(entry.usage?.input_tokens)}</Table.Cell>
									<Table.Cell class="text-right font-mono text-xs">{fmtTokens(entry.usage?.output_tokens)}</Table.Cell>
									<Table.Cell class="text-right font-mono text-xs">
										{fmtTokens(entry.usage ? entry.usage.cache_read_input_tokens + entry.usage.cache_creation_input_tokens : undefined)}
									</Table.Cell>
								</Table.Row>
								{#if expandedRows.has(entry.index)}
									<Table.Row>
										<Table.Cell colspan={8} class="bg-muted/30 p-0">
											<pre class="overflow-auto p-4 font-mono text-xs max-h-96">{formatJson(entry.raw)}</pre>
										</Table.Cell>
									</Table.Row>
								{/if}
							{/each}
						</Table.Body>
					</Table.Root>
				</Card.Content>
			</Card.Root>
		{/if}

		{#if diffFiles.length > 0}
			<Card.Root>
				<Card.Header class="pb-2">
					<Card.Title class="flex items-center gap-3">
						<span>Changes</span>
						<Badge variant="secondary">{diffSummary.fileCount} file{diffSummary.fileCount !== 1 ? 's' : ''}</Badge>
						<span class="text-sm font-normal">
							<span class="text-green-600">+{diffSummary.totalAdded}</span>
							<span class="text-red-600 ml-1">-{diffSummary.totalDeleted}</span>
						</span>
						{#if attrData}
							<Badge variant="outline" class="text-xs">
								{attrData.summary.ai_percentage.toFixed(0)}% AI
							</Badge>
						{/if}
					</Card.Title>
				</Card.Header>
				<Card.Content class="space-y-2 p-0">
					{#if !attrData}
						<div class="mx-4 mt-2 mb-2 rounded border border-blue-200 bg-blue-50 dark:border-blue-800 dark:bg-blue-950 p-3 text-sm text-blue-700 dark:text-blue-300">
							Attribution data not available. Install <a href="https://usegitai.com" class="underline font-medium" target="_blank" rel="noopener">git-ai</a> to track which lines were written by AI agents vs humans.
						</div>
					{/if}
					{#each diffFiles as file}
						<div class="border-t first:border-t-0">
							<button
								class="flex w-full items-center gap-2 px-4 py-2 text-left text-sm hover:bg-muted/50"
								onclick={() => toggleFile(file.path)}
							>
								<span class="text-muted-foreground">{expandedFiles.has(file.path) ? '▼' : '▶'}</span>
								<span class="font-mono font-medium">{file.path}</span>
								{#if file.old_path}
									<span class="text-muted-foreground text-xs">(renamed from {file.old_path})</span>
								{/if}
								<span class="ml-auto text-xs">
									<span class="text-green-600">+{fileAddedCount(file)}</span>
									<span class="text-red-600 ml-1">-{fileDeletedCount(file)}</span>
								</span>
								{#if attrData && fileAiLineCount(file.path) > 0}
									<Badge variant="outline" class="text-xs">AI: {fileAiLineCount(file.path)} lines</Badge>
								{:else if attrData}
									<span class="text-xs text-muted-foreground">Human only</span>
								{/if}
							</button>
							{#if expandedFiles.has(file.path)}
								<div class="overflow-x-auto">
									{#each file.hunks as hunk}
										<div class="bg-blue-50 dark:bg-blue-950/30 px-4 py-1 text-xs font-mono text-muted-foreground border-y">
											@@ -{hunk.old_start},{hunk.old_count} +{hunk.new_start},{hunk.new_count} @@
										</div>
										{#each hunk.lines as line}
											{@const isAi = line.kind === 'add' && line.new_line_number != null && isAiLine(file.path, line.new_line_number)}
											<div
												class="flex font-mono text-xs leading-5 {
													line.kind === 'delete'
														? 'bg-red-500/10'
														: line.kind === 'add'
															? isAi
																? 'bg-violet-500/10'
																: 'bg-green-500/10'
															: ''
												}"
											>
												<span class="w-12 shrink-0 select-none text-right pr-2 text-muted-foreground/50 border-r">
													{line.old_line_number ?? ''}
												</span>
												<span class="w-12 shrink-0 select-none text-right pr-2 text-muted-foreground/50 border-r">
													{line.new_line_number ?? ''}
												</span>
												<span class="w-5 shrink-0 select-none text-center {
													line.kind === 'add' ? 'text-green-600' : line.kind === 'delete' ? 'text-red-600' : 'text-muted-foreground/30'
												}">
													{line.kind === 'add' ? '+' : line.kind === 'delete' ? '-' : ' '}
												</span>
												<span class="whitespace-pre pl-1">{line.content}</span>
											</div>
										{/each}
									{/each}
								</div>
							{/if}
						</div>
					{/each}
				</Card.Content>
			</Card.Root>
		{/if}

		{#if trace.session_data}
			<details>
				<summary class="cursor-pointer text-sm text-muted-foreground hover:text-foreground">
					Session Data
				</summary>
				<pre class="mt-2 overflow-auto rounded bg-muted p-4 text-sm">{formatJson(trace.session_data)}</pre>
			</details>
		{/if}
	{/if}
</div>
