<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Table from '$lib/components/ui/table/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';

	interface SessionDetail {
		id: string;
		session_id: string;
		model: string | null;
		tool: string | null;
		total_tokens: number | null;
		input_tokens: number | null;
		output_tokens: number | null;
		estimated_cost_usd: number | null;
		api_calls: number | null;
		session_data: Record<string, unknown> | null;
		transcript: unknown[] | null;
		created_at: string;
	}

	interface CommitDetail {
		id: string;
		repo_id: string;
		commit_sha: string;
		branch: string | null;
		author: string;
		diff_data: FileDiff[] | null;
		attribution: Record<string, unknown> | null;
		created_at: string;
		sessions: SessionDetail[];
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
		let minTime = Infinity;
		let maxTime = -Infinity;
		const modelMap = new Map<string, { tokens: number; count: number }>();
		const toolCounts: Record<string, number> = {};

		for (const entry of entries) {
			if (entry.timestamp) {
				const t = new Date(entry.timestamp).getTime();
				if (t < minTime) minTime = t;
				if (t > maxTime) maxTime = t;
			}
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

		// Prefer timestamp range over turn_duration sum
		if (minTime !== Infinity && maxTime !== -Infinity && maxTime > minTime) {
			totalDurationMs = maxTime - minTime;
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

	function fmtTokens(n: number | undefined | null): string {
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
		if (ms < 60_000) return `${(ms / 1000).toFixed(1)}s`;
		if (ms < 3_600_000) return `${Math.floor(ms / 60_000)}m ${Math.floor((ms % 60_000) / 1000)}s`;
		return `${Math.floor(ms / 3_600_000)}h ${Math.floor((ms % 3_600_000) / 60_000)}m`;
	}

	let commit: CommitDetail | null = $state(null);
	let loading = $state(true);
	let error = $state('');
	let expandedSessions: Set<string> = $state(new Set());

	const commitId = $derived($page.params.id ?? '');

	onMount(async () => {
		try {
			commit = await api.get<CommitDetail>(`/api/v1/traces/${commitId}`);
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load commit';
		} finally {
			loading = false;
		}
	});

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleString();
	}

	function formatJson(obj: unknown): string {
		return JSON.stringify(obj, null, 2);
	}

	// Aggregate stats across all sessions
	const aggregatedStats = $derived.by(() => {
		if (!commit?.sessions) return null;
		let totalTokens = 0;
		let totalInput = 0;
		let totalOutput = 0;
		for (const s of commit.sessions) {
			totalTokens += s.total_tokens ?? 0;
			totalInput += s.input_tokens ?? 0;
			totalOutput += s.output_tokens ?? 0;
		}
		if (totalTokens === 0 && totalInput === 0 && totalOutput === 0) return null;
		return { totalTokens, totalInput, totalOutput };
	});

	// Per-session transcript data
	function sessionTranscript(session: SessionDetail) {
		if (!session.transcript) return { entries: [] as TranscriptEntry[], stats: computeStats([]) };
		const entries = parseTranscript(session.transcript);
		const stats = computeStats(entries);
		return { entries, stats };
	}

	function toggleSession(sessionId: string) {
		const next = new Set(expandedSessions);
		if (next.has(sessionId)) next.delete(sessionId);
		else next.add(sessionId);
		expandedSessions = next;
	}

	let expandedFiles: Set<string> = $state(new Set());

	const diffFiles = $derived.by(() => {
		if (!commit?.diff_data) return [] as FileDiff[];
		return commit.diff_data as FileDiff[];
	});

	const attrData = $derived.by(() => {
		if (!commit?.attribution) return null;
		return commit.attribution as unknown as AttributionData;
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
	<title>Commit Detail - TraceVault</title>
</svelte:head>

<div class="space-y-4">
	<div class="flex items-center gap-2">
		<a href="/traces" class="text-muted-foreground hover:underline">Commits</a>
		<span class="text-muted-foreground">/</span>
		<h1 class="text-2xl font-bold font-mono">{commitId.slice(0, 8)}</h1>
	</div>

	{#if loading}
		<p class="text-muted-foreground">Loading...</p>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if commit}
		<div class="grid gap-4 md:grid-cols-2">
			<Card.Root>
				<Card.Header>
					<Card.Title>Commit Info</Card.Title>
				</Card.Header>
				<Card.Content class="space-y-2">
					<div class="flex justify-between">
						<span class="text-muted-foreground">SHA</span>
						<span class="font-mono text-sm">{commit.commit_sha}</span>
					</div>
					{#if commit.branch}
						<div class="flex justify-between">
							<span class="text-muted-foreground">Branch</span>
							<Badge variant="outline">{commit.branch}</Badge>
						</div>
					{/if}
					<div class="flex justify-between">
						<span class="text-muted-foreground">Author</span>
						<span>{commit.author}</span>
					</div>
					<div class="flex justify-between">
						<span class="text-muted-foreground">Date</span>
						<span>{formatDate(commit.created_at)}</span>
					</div>
					<div class="flex justify-between">
						<span class="text-muted-foreground">Sessions</span>
						<span>{commit.sessions.length}</span>
					</div>
				</Card.Content>
			</Card.Root>

			{#if attrData}
				<Card.Root>
					<Card.Header>
						<Card.Title>AI Attribution</Card.Title>
					</Card.Header>
					<Card.Content class="space-y-2">
						<div class="flex justify-between">
							<span class="text-muted-foreground">AI Percentage</span>
							<span class="font-semibold">{attrData.summary.ai_percentage.toFixed(1)}%</span>
						</div>
						<div class="flex justify-between">
							<span class="text-muted-foreground">Human Percentage</span>
							<span class="font-semibold">{attrData.summary.human_percentage.toFixed(1)}%</span>
						</div>
						<div class="flex justify-between">
							<span class="text-muted-foreground">Lines Added</span>
							<span class="text-green-600">+{attrData.summary.total_lines_added}</span>
						</div>
						<div class="flex justify-between">
							<span class="text-muted-foreground">Lines Deleted</span>
							<span class="text-red-600">-{attrData.summary.total_lines_deleted}</span>
						</div>
					</Card.Content>
				</Card.Root>
			{/if}
		</div>

		{#if aggregatedStats}
			<div class="grid gap-4 md:grid-cols-3">
				<Card.Root>
					<Card.Header class="pb-2">
						<Card.Title class="text-sm font-medium text-muted-foreground">Total Tokens</Card.Title>
					</Card.Header>
					<Card.Content>
						<div class="text-2xl font-bold">{fmtTokens(aggregatedStats.totalTokens)}</div>
						<p class="text-xs text-muted-foreground">
							{fmtTokens(aggregatedStats.totalInput)} in / {fmtTokens(aggregatedStats.totalOutput)} out
						</p>
					</Card.Content>
				</Card.Root>

				<Card.Root>
					<Card.Header class="pb-2">
						<Card.Title class="text-sm font-medium text-muted-foreground">Sessions</Card.Title>
					</Card.Header>
					<Card.Content>
						<div class="text-2xl font-bold">{commit.sessions.length}</div>
					</Card.Content>
				</Card.Root>

				<Card.Root>
					<Card.Header class="pb-2">
						<Card.Title class="text-sm font-medium text-muted-foreground">Files Changed</Card.Title>
					</Card.Header>
					<Card.Content>
						<div class="text-2xl font-bold">{diffSummary.fileCount}</div>
						<p class="text-xs text-muted-foreground">
							<span class="text-green-600">+{diffSummary.totalAdded}</span>
							<span class="text-red-600 ml-1">-{diffSummary.totalDeleted}</span>
						</p>
					</Card.Content>
				</Card.Root>
			</div>
		{/if}

		{#if commit.sessions.length > 0}
			<h2 class="text-lg font-semibold">Sessions</h2>
			{#each commit.sessions as session (session.id)}
				{@const tx = sessionTranscript(session)}
				<Card.Root>
					<Card.Header>
						<button
							class="flex w-full items-center justify-between text-left"
							onclick={() => toggleSession(session.id)}
						>
							<div class="flex items-center gap-2">
								<span class="text-muted-foreground">{expandedSessions.has(session.id) ? '▼' : '▶'}</span>
								<Card.Title class="text-base">
									Session <span class="font-mono text-sm">{session.session_id}</span>
								</Card.Title>
								{#if session.model}
									<Badge variant="secondary">{session.model}</Badge>
								{/if}
								{#if session.tool}
									<Badge variant="outline">{session.tool}</Badge>
								{/if}
							</div>
							<div class="flex items-center gap-4 text-sm text-muted-foreground">
								<span>{fmtTokens(session.total_tokens)} tokens</span>
								<span>{new Date(session.created_at).toLocaleString()}</span>
							</div>
						</button>
					</Card.Header>

					{#if expandedSessions.has(session.id)}
						<Card.Content class="space-y-4">
							{#if tx.entries.length > 0}
								<div class="grid gap-4 md:grid-cols-4">
									<div>
										<p class="text-xs text-muted-foreground">Total Tokens</p>
										<p class="text-lg font-bold">{fmtTokens(tx.stats.totalInputTokens + tx.stats.totalOutputTokens)}</p>
										<p class="text-xs text-muted-foreground">
											{fmtTokens(tx.stats.totalInputTokens)} in / {fmtTokens(tx.stats.totalOutputTokens)} out
										</p>
									</div>
									<div>
										<p class="text-xs text-muted-foreground">Cache</p>
										<p class="text-lg font-bold">{fmtTokens(tx.stats.totalCacheReadTokens + tx.stats.totalCacheCreationTokens)}</p>
									</div>
									<div>
										<p class="text-xs text-muted-foreground">Turns</p>
										<p class="text-lg font-bold">{tx.stats.userMessageCount}</p>
										<p class="text-xs text-muted-foreground">{tx.stats.turnCount} events</p>
									</div>
									<div>
										<p class="text-xs text-muted-foreground">Duration</p>
										<p class="text-lg font-bold">{fmtDuration(tx.stats.totalDurationMs)}</p>
										<div class="flex flex-wrap gap-1 mt-1">
											{#each tx.stats.byModel as m}
												<Badge variant="outline" class="text-xs">{m.model} ({m.count})</Badge>
											{/each}
										</div>
									</div>
								</div>

								{#if Object.keys(tx.stats.toolUsageCounts).length > 0}
									<div>
										<p class="text-xs text-muted-foreground mb-1">Tool Usage</p>
										<div class="flex flex-wrap gap-1">
											{#each Object.entries(tx.stats.toolUsageCounts).sort((a, b) => b[1] - a[1]) as [tool, count]}
												<Badge variant="secondary" class="text-xs">{tool} ({count})</Badge>
											{/each}
										</div>
									</div>
								{/if}

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
										{#each tx.entries as entry (entry.index)}
											<Table.Row class="cursor-pointer hover:bg-muted/50">
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
										{/each}
									</Table.Body>
								</Table.Root>
							{/if}

							{#if session.session_data}
								<details>
									<summary class="cursor-pointer text-sm text-muted-foreground hover:text-foreground">
										Session Data
									</summary>
									<pre class="mt-2 overflow-auto rounded bg-muted p-4 text-sm max-h-96">{formatJson(session.session_data)}</pre>
								</details>
							{/if}
						</Card.Content>
					{/if}
				</Card.Root>
			{/each}
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

		{#if commit.attribution}
			<details>
				<summary class="cursor-pointer text-sm text-muted-foreground hover:text-foreground">
					Attribution Details
				</summary>
				<pre class="mt-2 overflow-auto rounded bg-muted p-4 text-sm">{formatJson(commit.attribution)}</pre>
			</details>
		{/if}
	{/if}
</div>
