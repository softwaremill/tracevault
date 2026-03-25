<script lang="ts">
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import * as Table from '$lib/components/ui/table/index.js';

	interface SessionInfo {
		id: string;
		session_id: string;
		repo_name: string;
		user_email: string | null;
		status: string;
		model: string | null;
		tool: string | null;
		total_tool_calls: number | null;
		total_tokens: number | null;
		estimated_cost_usd: number | null;
		cwd: string | null;
		started_at: string | null;
		ended_at: string | null;
		updated_at: string | null;
	}

	interface EventItem {
		id: string;
		event_index: number;
		event_type: string;
		tool_name: string | null;
		tool_input: unknown | null;
		tool_response: unknown | null;
		timestamp: string;
	}

	interface FileChange {
		id: string;
		file_path: string;
		change_type: string;
		diff_text: string | null;
		content_hash: string | null;
		timestamp: string;
	}

	interface TranscriptChunk {
		chunk_index: number;
		data: unknown;
	}

	interface LinkedCommit {
		commit_id: string;
		commit_sha: string;
		branch: string | null;
		confidence: number | null;
	}

	interface SessionDetailResponse {
		session: SessionInfo;
		events: EventItem[];
		file_changes: FileChange[];
		transcript_chunks: TranscriptChunk[];
		linked_commits: LinkedCommit[];
	}

	let data: SessionDetailResponse | null = $state(null);
	let loading = $state(true);
	let error = $state('');

	let expandedEvents = $state(new Set<string>());
	let expandedFiles = $state(new Set<string>());
	let expandedTools = $state(new Set<string>());
	let expandedResults = $state(new Set<string>());
	let sectionsOpen = $state({
		events: true,
		files: false,
		transcript: false,
		commits: false
	});

	const slug = $derived($page.params.slug);
	const sessionId = $derived($page.params.id);

	function displayStatus(session: SessionInfo): 'active' | 'completed' | 'stale' {
		if (session.status === 'completed') return 'completed';
		if (session.status === 'active' && session.updated_at) {
			const updatedAt = new Date(session.updated_at).getTime();
			const thirtyMinAgo = Date.now() - 30 * 60 * 1000;
			if (updatedAt < thirtyMinAgo) return 'stale';
		}
		return 'active';
	}

	const statusStyles: Record<string, { bg: string; text: string; label: string }> = {
		active: { bg: 'bg-green-500/15', text: 'text-green-600 dark:text-green-400', label: 'Active' },
		completed: { bg: 'bg-zinc-500/15', text: 'text-zinc-500 dark:text-zinc-400', label: 'Completed' },
		stale: { bg: 'bg-yellow-500/15', text: 'text-yellow-600 dark:text-yellow-400', label: 'Stale' }
	};

	const toolColors: Record<string, string> = {
		Edit: 'bg-amber-500',
		Write: 'bg-amber-500',
		Bash: 'bg-cyan-500',
		Read: 'bg-purple-500',
		Grep: 'bg-green-500',
		Glob: 'bg-indigo-500',
		Skill: 'bg-pink-500',
		Agent: 'bg-blue-500'
	};

	const toolBlockStyles: Record<string, { badge: string; badgeText: string; border: string }> = {
		Edit:  { badge: 'bg-amber-500/15',  badgeText: 'text-amber-600 dark:text-amber-400',   border: 'border-amber-500/30' },
		Write: { badge: 'bg-amber-500/15',  badgeText: 'text-amber-600 dark:text-amber-400',   border: 'border-amber-500/30' },
		Bash:  { badge: 'bg-cyan-500/15',   badgeText: 'text-cyan-600 dark:text-cyan-400',     border: 'border-cyan-500/30' },
		Read:  { badge: 'bg-purple-500/15', badgeText: 'text-purple-600 dark:text-purple-400', border: 'border-purple-500/30' },
		Grep:  { badge: 'bg-green-500/15',  badgeText: 'text-green-600 dark:text-green-400',   border: 'border-green-500/30' },
		Glob:  { badge: 'bg-indigo-500/15', badgeText: 'text-indigo-600 dark:text-indigo-400', border: 'border-indigo-500/30' },
		Skill: { badge: 'bg-pink-500/15',   badgeText: 'text-pink-600 dark:text-pink-400',     border: 'border-pink-500/30' },
		Agent: { badge: 'bg-blue-500/15',   badgeText: 'text-blue-600 dark:text-blue-400',     border: 'border-blue-500/30' }
	};

	const defaultToolBlockStyle = { badge: 'bg-gray-500/15', badgeText: 'text-gray-600 dark:text-gray-400', border: 'border-gray-500/30' };

	const FILE_MODIFYING_TOOLS = new Set(['Edit', 'Write', 'Bash']);

	function getToolColor(toolName: string | null): string {
		if (!toolName) return 'bg-zinc-400';
		for (const [key, color] of Object.entries(toolColors)) {
			if (toolName.toLowerCase().includes(key.toLowerCase())) return color;
		}
		return 'bg-zinc-400';
	}

	function eventSummary(event: EventItem): string {
		if (!event.tool_input) return '';
		const input = event.tool_input as Record<string, unknown>;
		if (input.file_path) return String(input.file_path);
		if (input.path) return String(input.path);
		if (input.command) {
			const cmd = String(input.command);
			return cmd.length > 80 ? cmd.slice(0, 80) + '...' : cmd;
		}
		if (input.pattern) return String(input.pattern);
		return '';
	}

	function toolSummary(name: string, input: Record<string, unknown>): string {
		switch (name) {
			case 'Edit':
			case 'Write':
			case 'Read':
				return input.file_path ? String(input.file_path) : '';
			case 'Bash':
				return input.command
					? String(input.command).slice(0, 100)
					: input.description
						? String(input.description)
						: '';
			case 'Grep':
				return `"${input.pattern ?? ''}" in ${input.path ?? '.'}`;
			case 'Glob':
				return input.pattern ? String(input.pattern) : '';
			case 'Skill':
				return input.skill ? String(input.skill) : '';
			case 'Agent':
				return input.description ? String(input.description) : '';
			default:
				return '';
		}
	}

	async function fetchDetail() {
		loading = true;
		error = '';
		try {
			data = await api.get<SessionDetailResponse>(
				`/api/v1/orgs/${slug}/traces/sessions/${sessionId}`
			);
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load session';
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		void slug;
		void sessionId;
		fetchDetail();
	});

	// Set initial expanded tools when data loads
	let lastDataId: string | null = null;
	$effect(() => {
		const sessionId = data?.session?.id ?? null;
		if (sessionId && sessionId !== lastDataId && data?.transcript_chunks) {
			lastDataId = sessionId;
			const { initialExpanded } = extractTurns(data.transcript_chunks);
			expandedTools = initialExpanded;
		}
	});

	function fmtNum(n: number | null): string {
		if (n == null) return '-';
		if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
		if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
		return String(n);
	}

	function fmtCost(n: number | null): string {
		if (n == null) return '-';
		return `$${n.toFixed(2)}`;
	}

	function fmtTime(iso: string | null): string {
		if (!iso) return '-';
		return new Date(iso).toLocaleString();
	}

	function fmtRelativeTime(iso: string): string {
		const diff = Date.now() - new Date(iso).getTime();
		const minutes = Math.floor(diff / 60000);
		const hours = Math.floor(minutes / 60);
		const days = Math.floor(hours / 24);
		if (days > 0) return `${days}d ago`;
		if (hours > 0) return `${hours}h ago`;
		if (minutes > 0) return `${minutes}m ago`;
		return 'just now';
	}

	function formatJson(obj: unknown): string {
		try {
			return JSON.stringify(obj, null, 2);
		} catch {
			return String(obj);
		}
	}

	function toggleEvent(id: string) {
		const next = new Set(expandedEvents);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		expandedEvents = next;
	}

	function toggleSection(key: keyof typeof sectionsOpen) {
		sectionsOpen = { ...sectionsOpen, [key]: !sectionsOpen[key] };
	}

	function toggleFile(id: string) {
		const next = new Set(expandedFiles);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		expandedFiles = next;
	}

	function toggleTool(id: string) {
		const next = new Set(expandedTools);
		if (next.has(id)) next.delete(id);
		else next.add(id);
		expandedTools = next;
	}

	function getInitialExpandedTools(toolUseMap: Map<string, ContentBlock & { type: 'tool_use' }>): Set<string> {
		const initial = new Set<string>();
		for (const [id, block] of toolUseMap) {
			if (FILE_MODIFYING_TOOLS.has(block.name)) {
				initial.add(id);
			}
		}
		return initial;
	}

	interface DiffLine {
		type: 'add' | 'remove' | 'header';
		content: string;
	}

	function parseDiff(diffText: string): DiffLine[] {
		const lines: DiffLine[] = [];
		for (const raw of diffText.split('\n')) {
			if (raw.startsWith('+++') || raw.startsWith('---')) {
				lines.push({ type: 'header', content: raw });
			} else if (raw.startsWith('+')) {
				lines.push({ type: 'add', content: raw });
			} else if (raw.startsWith('-')) {
				lines.push({ type: 'remove', content: raw });
			} else {
				// context line or empty
				lines.push({ type: 'header', content: raw });
			}
		}
		return lines;
	}

	type ContentBlock =
		| { type: 'text'; text: string }
		| { type: 'tool_use'; name: string; id: string; input: Record<string, unknown>; result?: string };

	interface TranscriptTurn {
		role: string;
		blocks: ContentBlock[];
	}

	function extractTurns(chunks: TranscriptChunk[]): { turns: TranscriptTurn[]; initialExpanded: Set<string> } {
		const turns: TranscriptTurn[] = [];
		const toolUseMap = new Map<string, ContentBlock & { type: 'tool_use' }>();

		for (const chunk of chunks) {
			if (!chunk.data) continue;
			const obj = chunk.data as Record<string, unknown>;

			let role: string | undefined;
			let content: unknown;

			const type = obj.type as string | undefined;
			if (type === 'user' || type === 'assistant') {
				role = type;
				const msg = obj.message as Record<string, unknown> | undefined;
				content = msg ? msg.content : obj.content;
			} else if (obj.role && (obj.role === 'user' || obj.role === 'assistant')) {
				role = String(obj.role);
				content = obj.content;
			}

			if (!role || content === undefined) continue;

			const blocks = extractBlocks(content, toolUseMap).filter(
				(b) => b.type !== 'text' || /\S/.test(b.text)
			);
			if (blocks.length > 0) {
				turns.push({ role, blocks });
			}
		}

		return { turns, initialExpanded: getInitialExpandedTools(toolUseMap) };
	}

	function extractBlocks(
		content: unknown,
		toolUseMap: Map<string, ContentBlock & { type: 'tool_use' }>
	): ContentBlock[] {
		if (typeof content === 'string') {
			return content.trim() ? [{ type: 'text', text: content }] : [];
		}
		if (Array.isArray(content)) {
			const blocks: ContentBlock[] = [];
			for (const c of content) {
				if (typeof c === 'string') {
					if (c.trim()) blocks.push({ type: 'text', text: c });
					continue;
				}
				if (c && typeof c === 'object') {
					const item = c as Record<string, unknown>;
					if (item.type === 'text' && item.text) {
						const text = String(item.text).trim();
						if (text) blocks.push({ type: 'text', text });
					} else if (item.type === 'tool_use') {
						const block: ContentBlock & { type: 'tool_use' } = {
							type: 'tool_use',
							name: String(item.name ?? 'Unknown'),
							id: String(item.id ?? ''),
							input: (item.input as Record<string, unknown>) ?? {}
						};
						blocks.push(block);
						if (block.id) toolUseMap.set(block.id, block);
					} else if (item.type === 'tool_result') {
						const useId = String(item.tool_use_id ?? '');
						const target = toolUseMap.get(useId);
						if (target) {
							target.result = flattenToolResult(item.content);
						}
					}
				}
			}
			return blocks;
		}
		if (content && typeof content === 'object') {
			return [{ type: 'text', text: JSON.stringify(content, null, 2) }];
		}
		return [];
	}

	function flattenToolResult(content: unknown): string {
		if (typeof content === 'string') return content;
		if (Array.isArray(content)) {
			return content
				.filter((r) => r && typeof r === 'object' && (r as Record<string, unknown>).type === 'text')
				.map((r) => String((r as Record<string, unknown>).text ?? ''))
				.join('\n');
		}
		return String(content ?? '');
	}
</script>

<svelte:head>
	<title>Session Detail - TraceVault</title>
</svelte:head>

<div class="space-y-5">
	{#if loading}
		<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm">
			<span
				class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
			></span>
			Loading session...
		</div>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if data}
		{@const session = data.session}
		{@const status = displayStatus(session)}
		{@const sc = statusStyles[status]}

		<!-- Breadcrumb + header -->
		<div class="flex items-center gap-3">
			<a
				href="/orgs/{slug}/traces/sessions"
				class="text-muted-foreground text-sm transition-colors hover:text-foreground"
			>
				Traces
			</a>
			<span class="text-muted-foreground/40">/</span>
			<a
				href="/orgs/{slug}/traces/sessions"
				class="text-muted-foreground text-sm transition-colors hover:text-foreground"
			>
				Sessions
			</a>
			<span class="text-muted-foreground/40">/</span>
			<span class="font-mono text-sm font-semibold">{session.session_id.slice(0, 8)}</span>
			<span class="inline-flex items-center rounded-full px-2 py-0.5 text-[10px] font-medium {sc.bg} {sc.text}">
				{sc.label}
			</span>
		</div>

		<!-- Session metadata -->
		<div class="border-border overflow-hidden rounded-lg border">
			<div class="bg-muted/30 flex flex-wrap items-center gap-x-6 gap-y-1 px-4 py-3 text-sm">
				<div class="flex items-center gap-2">
					<span class="text-muted-foreground text-xs uppercase tracking-wide">Session ID</span>
					<span class="font-mono text-xs">{session.session_id}</span>
				</div>
				<div class="flex items-center gap-2">
					<span class="text-muted-foreground text-xs uppercase tracking-wide">Repo</span>
					<span class="text-xs">{session.repo_name}</span>
				</div>
				{#if session.user_email}
					<div class="flex items-center gap-2">
						<span class="text-muted-foreground text-xs uppercase tracking-wide">User</span>
						<span class="text-xs">{session.user_email}</span>
					</div>
				{/if}
				{#if session.cwd}
					<div class="flex items-center gap-2">
						<span class="text-muted-foreground text-xs uppercase tracking-wide">CWD</span>
						<span class="font-mono text-xs">{session.cwd}</span>
					</div>
				{/if}
				<div class="flex items-center gap-2">
					<span class="text-muted-foreground text-xs uppercase tracking-wide">Started</span>
					<span class="text-xs">{fmtTime(session.started_at)}</span>
				</div>
			</div>

			<!-- Stats row -->
			<div class="grid grid-cols-2 gap-px md:grid-cols-5">
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Events</div>
					<div class="mt-1 text-lg font-semibold">{fmtNum(data.events.length)}</div>
				</div>
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Files</div>
					<div class="mt-1 text-lg font-semibold">{fmtNum(data.file_changes.length)}</div>
				</div>
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Tokens</div>
					<div class="mt-1 text-lg font-semibold">{fmtNum(session.total_tokens)}</div>
				</div>
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Cost</div>
					<div class="mt-1 text-lg font-semibold">{fmtCost(session.estimated_cost_usd)}</div>
				</div>
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Commits</div>
					<div class="mt-1 text-lg font-semibold">{fmtNum(data.linked_commits.length)}</div>
				</div>
			</div>
		</div>

		<!-- Accordion sections -->
		<div class="space-y-3">
			<!-- Events Timeline -->
			<div class="border-border overflow-hidden rounded-lg border">
				<button
					class="hover:bg-muted/40 flex w-full items-center gap-3 px-4 py-3 text-left transition-colors"
					onclick={() => toggleSection('events')}
				>
					<span class="text-muted-foreground/50 text-xs">{sectionsOpen.events ? '▼' : '▶'}</span>
					<span class="text-sm font-semibold">Events Timeline</span>
					<span class="text-muted-foreground ml-auto text-xs">{data.events.length} events</span>
				</button>
				{#if sectionsOpen.events}
					<div class="border-border border-t">
						{#if data.events.length === 0}
							<p class="text-muted-foreground px-4 py-4 text-sm">No events recorded.</p>
						{:else}
							<div class="divide-border divide-y">
								{#each data.events as event (event.id)}
									{@const color = getToolColor(event.tool_name)}
									<div>
										<button
											class="hover:bg-muted/30 flex w-full items-center gap-3 px-4 py-2 text-left text-xs transition-colors"
											onclick={() => toggleEvent(event.id)}
										>
											<span class="h-2.5 w-2.5 shrink-0 rounded-full {color}"></span>
											<span class="w-16 shrink-0 font-mono font-medium">{event.tool_name ?? event.event_type}</span>
											<span class="text-muted-foreground min-w-0 flex-1 truncate font-mono">{eventSummary(event)}</span>
											<span class="text-muted-foreground shrink-0">{fmtRelativeTime(event.timestamp)}</span>
										</button>
										{#if expandedEvents.has(event.id)}
											<div class="border-border border-t px-4 py-3">
												{#if event.tool_input}
													<div class="mb-2">
														<span class="text-muted-foreground text-[10px] uppercase tracking-wide">Input</span>
														<pre class="bg-muted/20 mt-1 max-h-60 overflow-auto rounded p-3 font-mono text-[11px] leading-relaxed">{formatJson(event.tool_input)}</pre>
													</div>
												{/if}
												{#if event.tool_response}
													<div>
														<span class="text-muted-foreground text-[10px] uppercase tracking-wide">Response</span>
														<pre class="bg-muted/20 mt-1 max-h-60 overflow-auto rounded p-3 font-mono text-[11px] leading-relaxed">{formatJson(event.tool_response)}</pre>
													</div>
												{/if}
											</div>
										{/if}
									</div>
								{/each}
							</div>
						{/if}
					</div>
				{/if}
			</div>

			<!-- File Changes -->
			<div class="border-border overflow-hidden rounded-lg border">
				<button
					class="hover:bg-muted/40 flex w-full items-center gap-3 px-4 py-3 text-left transition-colors"
					onclick={() => toggleSection('files')}
				>
					<span class="text-muted-foreground/50 text-xs">{sectionsOpen.files ? '▼' : '▶'}</span>
					<span class="text-sm font-semibold">File Changes</span>
					<span class="text-muted-foreground ml-auto text-xs">{data.file_changes.length} files</span>
				</button>
				{#if sectionsOpen.files}
					<div class="border-border border-t">
						{#if data.file_changes.length === 0}
							<p class="text-muted-foreground px-4 py-4 text-sm">No file changes recorded.</p>
						{:else}
							<div class="divide-border divide-y">
								{#each data.file_changes as fc (fc.id)}
									<div>
										<button
											class="hover:bg-muted/30 flex w-full items-center gap-2 px-4 py-2.5 text-left text-xs transition-colors"
											onclick={() => toggleFile(fc.id)}
										>
											<span class="text-muted-foreground/50 text-xs shrink-0">{expandedFiles.has(fc.id) ? '▼' : '▶'}</span>
											<span
												class="inline-flex shrink-0 rounded-full px-2 py-0.5 text-[10px] font-medium
													{fc.change_type === 'create'
														? 'bg-green-500/15 text-green-600 dark:text-green-400'
														: 'bg-amber-500/15 text-amber-600 dark:text-amber-400'}"
											>
												{fc.change_type}
											</span>
											<span class="font-mono font-medium">{fc.file_path.split('/').pop()}</span>
											<span class="text-muted-foreground truncate font-mono">{fc.file_path}</span>
										</button>
										{#if expandedFiles.has(fc.id)}
											<div class="border-border border-t bg-muted/30">
												{#if fc.diff_text}
													{@const diffLines = parseDiff(fc.diff_text)}
													<div class="overflow-x-auto">
														<table class="w-full border-collapse">
															<tbody>
																{#each diffLines as line, i}
																	<tr class="{line.type === 'add'
																		? 'bg-green-100 dark:bg-green-500/15'
																		: line.type === 'remove'
																			? 'bg-red-100 dark:bg-red-500/15'
																			: ''}">
																		<td class="select-none border-r border-border/40 px-2 text-right font-mono text-[10px] leading-relaxed {line.type === 'remove' ? 'text-red-400/60 dark:text-red-400/50' : line.type === 'add' ? 'text-green-500/60 dark:text-green-400/50' : 'text-muted-foreground/40'}" style="width: 1px; white-space: nowrap;">
																			{i + 1}
																		</td>
																		<td class="select-none border-r border-border/40 px-1.5 text-center font-mono text-[11px] leading-relaxed {line.type === 'add' ? 'text-green-600 dark:text-green-400' : line.type === 'remove' ? 'text-red-600 dark:text-red-400' : 'text-muted-foreground/40'}" style="width: 1px;">
																			{line.type === 'add' ? '+' : line.type === 'remove' ? '-' : ' '}
																		</td>
																		<td class="px-3 font-mono text-[11px] leading-relaxed whitespace-pre {line.type === 'add'
																			? 'text-green-800 dark:text-green-300'
																			: line.type === 'remove'
																				? 'text-red-800 dark:text-red-300'
																				: 'text-foreground'}">
																			{line.type === 'add' ? line.content.slice(1) : line.type === 'remove' ? line.content.slice(1) : line.type === 'header' && (line.content.startsWith('+++') || line.content.startsWith('---')) ? line.content : line.content}
																		</td>
																	</tr>
																{/each}
															</tbody>
														</table>
													</div>
												{:else}
													<div class="px-4 py-3 text-xs text-muted-foreground italic">
														{fc.change_type === 'create' ? 'New file created (no diff available)' : 'No diff data available'}
													</div>
												{/if}
											</div>
										{/if}
									</div>
								{/each}
							</div>
						{/if}
					</div>
				{/if}
			</div>

			<!-- Transcript -->
			<div class="border-border overflow-hidden rounded-lg border">
				<button
					class="hover:bg-muted/40 flex w-full items-center gap-3 px-4 py-3 text-left transition-colors"
					onclick={() => toggleSection('transcript')}
				>
					<span class="text-muted-foreground/50 text-xs">{sectionsOpen.transcript ? '▼' : '▶'}</span>
					<span class="text-sm font-semibold">Transcript</span>
					<span class="text-muted-foreground ml-auto text-xs">{data.transcript_chunks.length} chunks</span>
				</button>
				{#if sectionsOpen.transcript}
					<div class="border-border border-t">
						{#if data.transcript_chunks.length === 0}
							<p class="text-muted-foreground px-4 py-4 text-sm">No transcript data.</p>
						{:else}
							{@const extracted = extractTurns(data.transcript_chunks)}
							{#if extracted.turns.length === 0}
								<div class="divide-border divide-y">
									{#each data.transcript_chunks as chunk (chunk.chunk_index)}
										<pre class="max-h-60 overflow-auto px-4 py-3 font-mono text-[11px] leading-relaxed">{formatJson(chunk.data)}</pre>
									{/each}
								</div>
							{:else}
								<div class="space-y-1.5 p-4" style="outline: 2px solid blue;">
									{#each extracted.turns as turn}
									<div style="outline: 2px solid magenta; margin-bottom: 2px;">
										{#each turn.blocks as block}
											{#if block.type === 'text' && block.text.trim()}
												<div
													style="outline: 1px solid red;"
													class="max-w-[85%] rounded-lg px-3 py-2 text-xs
														{turn.role === 'user'
															? 'bg-primary/10 mr-auto'
															: turn.role === 'assistant'
																? 'bg-muted ml-auto'
																: 'bg-muted/50 mr-auto'}"
												>
													<div class="text-muted-foreground mb-1 text-[10px] font-medium uppercase">{turn.role}</div>
													<div class="whitespace-pre-wrap break-words">{block.text.trim().replace(/\n{3,}/g, '\n\n')}</div>
												</div>
											{:else if block.type === 'tool_use'}
												{@const style = toolBlockStyles[block.name] ?? defaultToolBlockStyle}
												{@const expanded = expandedTools.has(block.id)}
												{@const summary = toolSummary(block.name, block.input)}
												<div style="outline: 1px solid green;" class="rounded-lg border overflow-hidden {style.border}">
													<button
														class="flex w-full items-center gap-2.5 px-3 py-2 text-left text-xs transition-colors hover:bg-muted/30"
														onclick={() => toggleTool(block.id)}
													>
														<span class="text-muted-foreground text-[10px]">{expanded ? '▼' : '▶'}</span>
														<span class="shrink-0 rounded-full px-2 py-0.5 text-[10px] font-semibold {style.badge} {style.badgeText}">
															{block.name}
														</span>
														{#if summary}
															<span class="text-muted-foreground min-w-0 flex-1 truncate font-mono text-[11px]">{summary}</span>
														{/if}
													</button>
													{#if expanded}
														<div class="border-t border-border/50 px-3 py-2.5 space-y-2">
															{#if Object.keys(block.input).length > 0}
																<div>
																	<span class="text-muted-foreground text-[10px] uppercase tracking-wide">Input</span>
																	<pre class="bg-muted/20 mt-1 max-h-60 overflow-auto rounded p-2.5 font-mono text-[11px] leading-relaxed">{formatJson(block.input)}</pre>
																</div>
															{/if}
															{#if block.result}
																{@const truncated = block.result.length > 500}
																{@const resultId = block.id + '-result'}
																<div>
																	<span class="text-muted-foreground text-[10px] uppercase tracking-wide">Result</span>
																	<pre class="bg-muted/20 mt-1 max-h-60 overflow-auto rounded p-2.5 font-mono text-[11px] leading-relaxed">{truncated && !expandedResults.has(resultId) ? block.result.slice(0, 500) + '...' : block.result}</pre>
																	{#if truncated}
																		<button
																			class="text-[10px] text-muted-foreground hover:text-foreground mt-1"
																			onclick={() => {
																				const next = new Set(expandedResults);
																				if (next.has(resultId)) next.delete(resultId);
																				else next.add(resultId);
																				expandedResults = next;
																			}}
																		>
																			{expandedResults.has(resultId) ? 'Show less' : 'Show more'}
																		</button>
																	{/if}
																</div>
															{/if}
														</div>
													{/if}
												</div>
											{/if}
										{/each}
									</div>
									{/each}
								</div>
							{/if}
						{/if}
					</div>
				{/if}
			</div>

			<!-- Linked Commits -->
			<div class="border-border overflow-hidden rounded-lg border">
				<button
					class="hover:bg-muted/40 flex w-full items-center gap-3 px-4 py-3 text-left transition-colors"
					onclick={() => toggleSection('commits')}
				>
					<span class="text-muted-foreground/50 text-xs">{sectionsOpen.commits ? '▼' : '▶'}</span>
					<span class="text-sm font-semibold">Linked Commits</span>
					<span class="text-muted-foreground ml-auto text-xs">{data.linked_commits.length} commits</span>
				</button>
				{#if sectionsOpen.commits}
					<div class="border-border border-t">
						{#if data.linked_commits.length === 0}
							<p class="text-muted-foreground px-4 py-4 text-sm">No linked commits.</p>
						{:else}
							<Table.Root class="text-xs">
								<Table.Header>
									<Table.Row>
										<Table.Head>Commit SHA</Table.Head>
										<Table.Head>Branch</Table.Head>
										<Table.Head>Confidence</Table.Head>
									</Table.Row>
								</Table.Header>
								<Table.Body>
									{#each data.linked_commits as commit (commit.commit_id)}
										<Table.Row class="hover:bg-muted/40 transition-colors">
											<Table.Cell>
												<a
													href="/orgs/{slug}/traces/commits/{commit.commit_id}"
													class="font-mono text-sm underline"
												>
													{commit.commit_sha.slice(0, 8)}
												</a>
											</Table.Cell>
											<Table.Cell>
												{#if commit.branch}
													<span
														class="inline-flex rounded-full border-transparent bg-blue-100 px-2 py-0.5 text-[10px] font-medium text-blue-700 dark:bg-blue-950 dark:text-blue-300"
													>
														{commit.branch}
													</span>
												{:else}
													<span class="text-muted-foreground">-</span>
												{/if}
											</Table.Cell>
											<Table.Cell class="font-mono text-sm">
												{#if commit.confidence != null}
													{(commit.confidence * 100).toFixed(0)}%
												{:else}
													-
												{/if}
											</Table.Cell>
										</Table.Row>
									{/each}
								</Table.Body>
							</Table.Root>
						{/if}
					</div>
				{/if}
			</div>
		</div>
	{/if}
</div>
