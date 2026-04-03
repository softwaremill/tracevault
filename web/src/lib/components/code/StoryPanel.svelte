<script lang="ts">
	import type { StoryResponse, FunctionSessionsResponse } from '$lib/types/code';
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import { marked } from 'marked';
	import { formatDateTime } from '$lib/utils/date';

	let {
		story,
		loading,
		error = '',
		repoId,
		selectedLine,
		gitRef,
		filePath,
		onClose,
		onGenerateStory,
		onRegenerateStory
	}: {
		story: StoryResponse | null;
		loading: boolean;
		error?: string;
		repoId: string;
		selectedLine: number | null;
		gitRef: string;
		filePath: string;
		onClose: () => void;
		onGenerateStory: () => void;
		onRegenerateStory: () => void;
	} = $props();

	const slug = $derived($page.params.slug);

	// Tab state — sessions is default
	let activeTab = $state<'sessions' | 'story'>('sessions');

	// Sessions state
	let sessions = $state<FunctionSessionsResponse | null>(null);
	let sessionsLoading = $state(false);
	let sessionsError = $state('');

	// Reset when line changes — default to sessions tab
	$effect(() => {
		selectedLine;
		activeTab = 'sessions';
		sessions = null;
		sessionsError = '';
	});

	// Fetch sessions when sessions tab is active
	$effect(() => {
		if (activeTab === 'sessions' && selectedLine != null) {
			fetchSessions();
		}
	});

	// Generate story when story tab is first opened
	$effect(() => {
		if (activeTab === 'story' && selectedLine != null && !story && !loading) {
			onGenerateStory();
		}
	});

	async function fetchSessions() {
		sessionsLoading = true;
		sessionsError = '';
		sessions = null;
		try {
			const params = new URLSearchParams({
				ref: gitRef,
				path: filePath,
				line: String(selectedLine)
			});
			sessions = await api.get<FunctionSessionsResponse>(
				`/api/v1/orgs/${slug}/repos/${repoId}/code/sessions?${params}`
			);
		} catch (e) {
			sessionsError = e instanceof Error ? e.message : 'Failed to load sessions';
		}
		sessionsLoading = false;
	}

	// Build a SHA→trace_id lookup from references
	const shaToTraceId = $derived.by(() => {
		const map = new Map<string, string>();
		for (const ref of story?.references ?? []) {
			if (!ref.id) continue;
			map.set(ref.sha, ref.id);
			map.set(ref.sha.slice(0, 7), ref.id);
			for (const len of [8, 10, 12]) {
				if (ref.sha.length >= len) {
					map.set(ref.sha.slice(0, len), ref.id);
				}
			}
		}
		return map;
	});

	// Build session ID prefix → session UUID lookup from references
	const sessionIdToUuid = $derived.by(() => {
		const map = new Map<string, string>();
		for (const ref of story?.references ?? []) {
			for (const session of ref.sessions) {
				// Map the 8-char prefix of the UUID (what the LLM references)
				map.set(session.id.slice(0, 8), session.id);
			}
		}
		return map;
	});

	// Render markdown then inject commit and session links into the HTML
	const renderedMarkdown = $derived.by(() => {
		if (!story) return '';
		let html = marked.parse(story.story) as string;
		// Linkify commit SHAs
		if (shaToTraceId.size > 0) {
			html = html.replace(/\b([0-9a-f]{7,40})\b/g, (match) => {
				const traceId = shaToTraceId.get(match);
				if (!traceId) return match;
				return `<a href="/orgs/${slug}/traces/${traceId}" class="commit-link">${match}</a>`;
			});
		}
		// Linkify session ID prefixes (8-char hex patterns that match known sessions)
		if (sessionIdToUuid.size > 0) {
			html = html.replace(/\b([0-9a-f]{8})\b/g, (match) => {
				// Skip if already linkified as a commit
				const sessionUuid = sessionIdToUuid.get(match);
				if (!sessionUuid) return match;
				// Don't double-linkify (check if already inside an <a> tag)
				return `<a href="/orgs/${slug}/traces/sessions/${sessionUuid}" class="session-link">${match}</a>`;
			});
		}
		return html;
	});

	const trackedRefs = $derived(
		(story?.references ?? []).filter((r) => r.id !== null)
	);

	const panelVisible = $derived(selectedLine != null);
</script>

{#if panelVisible}
	<div class="fixed right-0 top-0 z-50 flex h-full w-[480px] flex-col border-l bg-card shadow-xl">
		<div class="flex items-center justify-between border-b p-4">
			<div>
				<h3 class="text-lg font-semibold">
					{#if story}
						{story.function_name}
					{:else if sessions}
						{sessions.function_name}
					{:else}
						{#if loading}Generating story...{:else}Code Sessions{/if}
					{/if}
				</h3>
				{#if story}
					<p class="text-sm text-muted-foreground">
						{story.kind} -- Lines {story.line_range[0]}-{story.line_range[1]}
						{#if story.cached}-- Cached{/if}
					</p>
				{:else if sessions}
					<p class="text-sm text-muted-foreground">
						Lines {sessions.line_range[0]}-{sessions.line_range[1]}
					</p>
				{/if}
			</div>
			<div class="flex gap-2">
				{#if story && activeTab === 'story'}
					<button
						onclick={onRegenerateStory}
						class="rounded border px-2 py-1 text-sm hover:bg-accent"
					>
						Regenerate
					</button>
				{/if}
				<button onclick={onClose} class="rounded border px-2 py-1 text-sm hover:bg-accent">
					X
				</button>
			</div>
		</div>

		<!-- Tabs -->
		<div class="flex border-b">
			<button
				class="flex-1 px-4 py-2 text-sm font-medium transition-colors {activeTab === 'sessions'
					? 'border-b-2 border-primary text-primary'
					: 'text-muted-foreground hover:text-foreground'}"
				onclick={() => (activeTab = 'sessions')}
			>
				Sessions
			</button>
			<button
				class="flex-1 px-4 py-2 text-sm font-medium transition-colors {activeTab === 'story'
					? 'border-b-2 border-primary text-primary'
					: 'text-muted-foreground hover:text-foreground'}"
				onclick={() => (activeTab = 'story')}
			>
				Story
			</button>
		</div>

		<div class="flex-1 overflow-y-auto p-4">
			{#if activeTab === 'sessions'}
				{#if sessionsLoading}
					<div class="flex items-center gap-2 text-muted-foreground">
						<div
							class="h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
						></div>
						Loading sessions...
					</div>
				{:else if sessionsError}
					<div class="rounded-md border border-destructive/50 bg-destructive/10 p-3 text-sm text-destructive">
						{sessionsError}
					</div>
				{:else if sessions && sessions.sessions.length === 0}
					<p class="text-sm text-muted-foreground">No sessions found for this function.</p>
				{:else if sessions}
					<div class="space-y-2">
						{#each sessions.sessions as session}
							<a
								href="/orgs/{slug}/traces/sessions/{session.id}"
								class="block rounded border bg-muted/30 px-3 py-2 text-sm transition-colors hover:bg-muted/50"
							>
								<div class="flex items-center justify-between">
									<span class="font-medium text-foreground">
										{session.started_at ? formatDateTime(session.started_at) : 'Unknown date'}
									</span>
									{#if session.model}
										<span class="text-xs text-muted-foreground">{session.model}</span>
									{/if}
								</div>
								{#if session.user_email}
									<p class="mt-0.5 text-xs text-muted-foreground">{session.user_email}</p>
								{/if}
								{#if session.commit_shas.length > 0}
									<div class="mt-1 flex flex-wrap gap-1">
										{#each session.commit_shas as sha}
											<span class="rounded bg-primary/10 px-1.5 py-0.5 font-mono text-[10px] text-primary">
												{sha.slice(0, 7)}
											</span>
										{/each}
									</div>
								{/if}
							</a>
						{/each}
					</div>
				{:else}
					<p class="text-sm text-muted-foreground">Select a line to view associated sessions.</p>
				{/if}
			{:else}
				<!-- Story tab -->
				{#if loading}
					<div class="flex items-center gap-2 text-muted-foreground">
						<div
							class="h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
						></div>
						Analyzing code history and generating story...
					</div>
				{:else if error}
					<div class="space-y-3">
						<div class="rounded-md border border-destructive/50 bg-destructive/10 p-3 text-sm text-destructive">
							{error}
						</div>
						<p class="text-sm text-muted-foreground">
							This usually means the LLM provider is not configured. Go to
							<a href="/orgs/{slug}/settings/llm" class="underline font-medium text-foreground">Settings &rarr; LLM</a>
							to configure your provider and API key.
						</p>
					</div>
				{:else if story}
					<article class="story-markdown">
						{@html renderedMarkdown}
					</article>

					{#if trackedRefs.length > 0}
						<div class="mt-6 border-t pt-4">
							<h4 class="mb-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground">
								Related Traces
							</h4>
							<div class="space-y-2">
								{#each trackedRefs as ref}
									<div class="rounded border bg-muted/30 px-3 py-2 text-xs">
										<div class="flex items-center gap-2">
											<a
												href="/orgs/{slug}/traces/{ref.id}"
												class="font-mono font-medium text-foreground underline decoration-muted-foreground/50 underline-offset-2 hover:decoration-foreground"
											>
												{ref.sha.slice(0, 7)}
											</a>
											<span class="truncate text-muted-foreground">{ref.message}</span>
										</div>
										{#if ref.sessions.length > 0}
											<div class="mt-1 flex flex-wrap gap-1">
												{#each ref.sessions as session}
													<a
														href="/orgs/{slug}/traces/{ref.id}"
														class="inline-flex items-center gap-1 rounded bg-primary/10 px-1.5 py-0.5 text-[10px] font-medium text-primary hover:bg-primary/20"
													>
														<span>Session {session.session_id.slice(0, 8)}</span>
														{#if session.model}
															<span class="text-muted-foreground">({session.model})</span>
														{/if}
													</a>
												{/each}
											</div>
										{/if}
									</div>
								{/each}
							</div>
						</div>
					{/if}

					<div class="mt-4 space-y-1 border-t pt-4 text-xs text-muted-foreground">
						<p>Commits analyzed: {story.commits_analyzed.length}</p>
						<p>Generated: {formatDateTime(story.generated_at)}</p>
					</div>
				{/if}
			{/if}
		</div>
	</div>
{/if}

<style>
	.story-markdown {
		font-size: 0.875rem;
		line-height: 1.7;
		color: var(--foreground);
	}

	.story-markdown :global(h1) {
		font-size: 1.5rem;
		font-weight: 700;
		margin-top: 1.5rem;
		margin-bottom: 0.75rem;
		line-height: 1.3;
		border-bottom: 1px solid var(--border);
		padding-bottom: 0.5rem;
	}

	.story-markdown :global(h2) {
		font-size: 1.25rem;
		font-weight: 600;
		margin-top: 1.5rem;
		margin-bottom: 0.5rem;
		line-height: 1.3;
	}

	.story-markdown :global(h3) {
		font-size: 1.1rem;
		font-weight: 600;
		margin-top: 1.25rem;
		margin-bottom: 0.5rem;
		line-height: 1.4;
	}

	.story-markdown :global(h4) {
		font-size: 1rem;
		font-weight: 600;
		margin-top: 1rem;
		margin-bottom: 0.375rem;
	}

	.story-markdown :global(p) {
		margin-top: 0.625rem;
		margin-bottom: 0.625rem;
	}

	.story-markdown :global(> :first-child) {
		margin-top: 0;
	}

	.story-markdown :global(ul),
	.story-markdown :global(ol) {
		margin-top: 0.5rem;
		margin-bottom: 0.5rem;
		padding-left: 1.5rem;
	}

	.story-markdown :global(ul) {
		list-style-type: disc;
	}

	.story-markdown :global(ol) {
		list-style-type: decimal;
	}

	.story-markdown :global(li) {
		margin-top: 0.25rem;
		margin-bottom: 0.25rem;
	}

	.story-markdown :global(li > ul),
	.story-markdown :global(li > ol) {
		margin-top: 0.125rem;
		margin-bottom: 0.125rem;
	}

	.story-markdown :global(strong) {
		font-weight: 600;
	}

	.story-markdown :global(em) {
		font-style: italic;
	}

	.story-markdown :global(a) {
		color: var(--primary);
		text-decoration: underline;
		text-underline-offset: 2px;
	}

	.story-markdown :global(a:hover) {
		opacity: 0.8;
	}

	.story-markdown :global(a.commit-link) {
		font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, monospace;
		font-size: 0.8em;
		background: var(--primary);
		color: var(--primary-foreground);
		padding: 0.1rem 0.375rem;
		border-radius: 0.25rem;
		text-decoration: none;
		white-space: nowrap;
	}

	.story-markdown :global(a.commit-link:hover) {
		opacity: 0.85;
	}

	.story-markdown :global(a.session-link) {
		font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, monospace;
		font-size: 0.8em;
		background: var(--muted);
		color: var(--foreground);
		padding: 0.1rem 0.375rem;
		border-radius: 0.25rem;
		text-decoration: none;
		white-space: nowrap;
		border: 1px solid var(--border);
	}

	.story-markdown :global(a.session-link:hover) {
		background: var(--accent);
	}

	.story-markdown :global(code) {
		font-family: ui-monospace, SFMono-Regular, 'SF Mono', Menlo, monospace;
		font-size: 0.8em;
		background: var(--muted);
		padding: 0.125rem 0.375rem;
		border-radius: 0.25rem;
	}

	.story-markdown :global(pre) {
		margin-top: 0.75rem;
		margin-bottom: 0.75rem;
		padding: 0.75rem 1rem;
		background: var(--muted);
		border-radius: 0.375rem;
		overflow-x: auto;
		font-size: 0.8rem;
		line-height: 1.6;
	}

	.story-markdown :global(pre code) {
		background: none;
		padding: 0;
		border-radius: 0;
		font-size: inherit;
	}

	.story-markdown :global(blockquote) {
		margin-top: 0.75rem;
		margin-bottom: 0.75rem;
		padding-left: 1rem;
		border-left: 3px solid var(--border);
		color: var(--muted-foreground);
		font-style: italic;
	}

	.story-markdown :global(hr) {
		margin-top: 1.5rem;
		margin-bottom: 1.5rem;
		border: none;
		border-top: 1px solid var(--border);
	}

	.story-markdown :global(table) {
		width: 100%;
		margin-top: 0.75rem;
		margin-bottom: 0.75rem;
		border-collapse: collapse;
		font-size: 0.8rem;
	}

	.story-markdown :global(th) {
		text-align: left;
		font-weight: 600;
		padding: 0.375rem 0.75rem;
		border-bottom: 2px solid var(--border);
	}

	.story-markdown :global(td) {
		padding: 0.375rem 0.75rem;
		border-bottom: 1px solid var(--border);
	}
</style>
