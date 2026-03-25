<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { api } from '$lib/api';
	import hljs from 'highlight.js/lib/core';
	import rust from 'highlight.js/lib/languages/rust';
	import typescript from 'highlight.js/lib/languages/typescript';
	import javascript from 'highlight.js/lib/languages/javascript';
	import python from 'highlight.js/lib/languages/python';
	import go from 'highlight.js/lib/languages/go';
	import java from 'highlight.js/lib/languages/java';
	import scala from 'highlight.js/lib/languages/scala';
	import json from 'highlight.js/lib/languages/json';
	import yaml from 'highlight.js/lib/languages/yaml';
	import sql from 'highlight.js/lib/languages/sql';
	import bash from 'highlight.js/lib/languages/bash';
	import css from 'highlight.js/lib/languages/css';
	import xml from 'highlight.js/lib/languages/xml';
	import markdown from 'highlight.js/lib/languages/markdown';
	import 'highlight.js/styles/github-dark.css';

	hljs.registerLanguage('rust', rust);
	hljs.registerLanguage('typescript', typescript);
	hljs.registerLanguage('javascript', javascript);
	hljs.registerLanguage('python', python);
	hljs.registerLanguage('go', go);
	hljs.registerLanguage('java', java);
	hljs.registerLanguage('scala', scala);
	hljs.registerLanguage('json', json);
	hljs.registerLanguage('yaml', yaml);
	hljs.registerLanguage('sql', sql);
	hljs.registerLanguage('bash', bash);
	hljs.registerLanguage('css', css);
	hljs.registerLanguage('xml', xml);
	hljs.registerLanguage('markdown', markdown);

	interface Commit {
		id: string;
		commit_sha: string;
		branch: string | null;
		author: string;
		committed_at: string;
	}

	interface FileAttribution {
		file_path: string;
		sessions: { session_id: string; session_short_id: string; confidence: number; line_start: number; line_end: number }[];
	}

	interface CommitDetail {
		commit: Commit;
		diff_data: unknown;
		attributions_by_file: FileAttribution[];
	}

	interface AttributionLine {
		line_number: number;
		content: string;
		git_author: string | null;
		session_id: string | null;
		session_short_id: string | null;
		confidence: number | null;
	}

	interface AttributionResponse {
		file_path: string;
		commit_sha: string;
		lines: AttributionLine[];
	}

	const slug = $derived($page.params.slug);

	// State
	let commits = $state<Commit[]>([]);
	let commitsLoading = $state(false);
	let commitsError = $state('');

	let selectedCommitId = $state<string>('');
	let filePaths = $state<string[]>([]);
	let filesLoading = $state(false);

	let selectedFilePath = $state<string>('');
	let attribution = $state<AttributionResponse | null>(null);
	let attributionLoading = $state(false);
	let attributionError = $state('');

	// Read initial values from URL params
	const initialCommitId = $page.url.searchParams.get('commit_id') ?? '';
	const initialFile = $page.url.searchParams.get('file') ?? '';
	if (initialCommitId) selectedCommitId = initialCommitId;
	if (initialFile) selectedFilePath = initialFile;

	// Derive language from file extension
	const language = $derived.by(() => {
		if (!selectedFilePath) return null;
		const ext = selectedFilePath.split('.').pop()?.toLowerCase();
		const map: Record<string, string> = {
			rs: 'rust',
			ts: 'typescript',
			tsx: 'typescript',
			js: 'javascript',
			jsx: 'javascript',
			py: 'python',
			go: 'go',
			java: 'java',
			scala: 'scala',
			json: 'json',
			yml: 'yaml',
			yaml: 'yaml',
			sql: 'sql',
			sh: 'bash',
			bash: 'bash',
			zsh: 'bash',
			css: 'css',
			html: 'xml',
			xml: 'xml',
			svg: 'xml',
			md: 'markdown',
			svelte: 'xml'
		};
		return ext ? (map[ext] ?? null) : null;
	});

	// Highlighted lines
	const highlightedLines = $derived.by(() => {
		if (!attribution) return [];
		const raw = attribution.lines.map((l) => l.content);
		const fullContent = raw.join('\n');
		let highlighted: string;
		if (language && hljs.getLanguage(language)) {
			highlighted = hljs.highlight(fullContent, { language }).value;
		} else {
			highlighted = hljs.highlightAuto(fullContent).value;
		}
		return highlighted.split('\n');
	});

	// Unique sessions in current file
	const uniqueSessions = $derived.by(() => {
		if (!attribution) return [];
		const seen = new Map<string, string>();
		for (const line of attribution.lines) {
			if (line.session_id && line.session_short_id && !seen.has(line.session_id)) {
				seen.set(line.session_id, line.session_short_id);
			}
		}
		return Array.from(seen.entries()).map(([id, shortId]) => ({ session_id: id, session_short_id: shortId }));
	});

	// Fetch commits list
	async function fetchCommits() {
		commitsLoading = true;
		commitsError = '';
		try {
			commits = await api.get<Commit[]>(`/api/v1/orgs/${slug}/traces/commits`);
		} catch (err) {
			commitsError = err instanceof Error ? err.message : 'Failed to load commits';
		} finally {
			commitsLoading = false;
		}
	}

	// Fetch file paths for a commit
	async function fetchFilePaths(commitId: string) {
		filesLoading = true;
		filePaths = [];
		try {
			const detail = await api.get<CommitDetail>(`/api/v1/orgs/${slug}/traces/commits/${commitId}`);
			filePaths = detail.attributions_by_file.map((f) => f.file_path);
		} catch {
			filePaths = [];
		} finally {
			filesLoading = false;
		}
	}

	// Fetch attribution data
	async function fetchAttribution(commitId: string, filePath: string) {
		attributionLoading = true;
		attributionError = '';
		attribution = null;
		try {
			attribution = await api.get<AttributionResponse>(
				`/api/v1/orgs/${slug}/traces/attribution/${commitId}/${encodeURIComponent(filePath)}`
			);
		} catch (err) {
			attributionError = err instanceof Error ? err.message : 'Failed to load attribution';
		} finally {
			attributionLoading = false;
		}
	}

	// Update URL params
	function updateUrl() {
		const params = new URLSearchParams();
		if (selectedCommitId) params.set('commit_id', selectedCommitId);
		if (selectedFilePath) params.set('file', selectedFilePath);
		const qs = params.toString();
		goto(`?${qs}`, { replaceState: true, keepFocus: true });
	}

	// Load commits on mount
	$effect(() => {
		fetchCommits();
	});

	// When commit changes, fetch file paths
	$effect(() => {
		if (selectedCommitId) {
			fetchFilePaths(selectedCommitId);
		} else {
			filePaths = [];
			selectedFilePath = '';
			attribution = null;
		}
	});

	// When file changes, fetch attribution
	$effect(() => {
		if (selectedCommitId && selectedFilePath) {
			fetchAttribution(selectedCommitId, selectedFilePath);
			updateUrl();
		} else {
			attribution = null;
		}
	});

	function onCommitChange(e: Event) {
		const target = e.target as HTMLSelectElement;
		selectedCommitId = target.value;
		selectedFilePath = '';
		attribution = null;
	}

	function onFileChange(e: Event) {
		const target = e.target as HTMLSelectElement;
		selectedFilePath = target.value;
	}
</script>

<svelte:head>
	<title>Attribution - TraceVault</title>
</svelte:head>

<div class="space-y-4">
	<h1 class="text-2xl font-bold">Code Attribution</h1>

	<!-- Selectors -->
	<div class="flex flex-wrap items-end gap-4">
		<!-- Commit selector -->
		<div class="flex flex-col gap-1">
			<label for="commit-select" class="text-muted-foreground text-xs uppercase tracking-wide">Commit</label>
			<select
				id="commit-select"
				class="border-border bg-background text-foreground rounded-md border px-3 py-1.5 font-mono text-sm"
				value={selectedCommitId}
				onchange={onCommitChange}
				disabled={commitsLoading}
			>
				<option value="">Select a commit...</option>
				{#each commits as c (c.id)}
					<option value={c.id}>
						{c.commit_sha.slice(0, 8)}{c.branch ? ` (${c.branch})` : ''} - {c.author}
					</option>
				{/each}
			</select>
			{#if commitsError}
				<span class="text-destructive text-xs">{commitsError}</span>
			{/if}
		</div>

		<!-- File selector -->
		<div class="flex flex-col gap-1">
			<label for="file-select" class="text-muted-foreground text-xs uppercase tracking-wide">File</label>
			<select
				id="file-select"
				class="border-border bg-background text-foreground rounded-md border px-3 py-1.5 font-mono text-sm"
				value={selectedFilePath}
				onchange={onFileChange}
				disabled={!selectedCommitId || filesLoading}
			>
				<option value="">Select a file...</option>
				{#each filePaths as fp}
					<option value={fp}>{fp}</option>
				{/each}
			</select>
			{#if filesLoading}
				<span class="text-muted-foreground text-xs">Loading files...</span>
			{/if}
		</div>
	</div>

	<!-- Loading / Error states -->
	{#if attributionLoading}
		<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm">
			<span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"></span>
			Loading attribution...
		</div>
	{:else if attributionError}
		<p class="text-destructive">{attributionError}</p>
	{:else if attribution && attribution.lines.length > 0}
		<!-- Code blame view -->
		<div class="code-blame hljs overflow-x-auto rounded-lg font-mono text-sm">
			<table class="w-full border-collapse">
				<tbody>
					{#each attribution.lines as line, i}
						{@const hasAi = !!line.session_id}
						<tr class={hasAi ? 'bg-ai-line' : ''}>
							<!-- Line number -->
							<td class="line-number w-12 select-none border-r border-gray-700 py-0 pr-3 text-right">
								{line.line_number}
							</td>
							<!-- Attribution column -->
							<td class="attribution-col w-[120px] min-w-[120px] select-none border-r border-gray-700 py-0 px-2 text-xs">
								{#if line.session_id && line.session_short_id}
									<a
										href="/orgs/{slug}/traces/sessions/{line.session_id}"
										class="attribution-session font-medium hover:underline"
									>
										{line.session_short_id}
									</a>
									{#if line.confidence != null}
										<span class="attribution-confidence ml-1">{(line.confidence * 100).toFixed(0)}%</span>
									{/if}
								{:else if line.git_author}
									<span class="attribution-author">{line.git_author}</span>
								{/if}
							</td>
							<!-- Code content -->
							<td class="whitespace-pre py-0 pl-4">
								{@html highlightedLines[i] || ' '}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>

		<!-- Legend -->
		{#if uniqueSessions.length > 0}
			<div class="flex flex-wrap items-center gap-4 pt-2 text-xs">
				{#each uniqueSessions as s}
					<div class="flex items-center gap-2">
						<span class="inline-block h-3 w-3 rounded" style="background: rgba(99,102,241,0.25); border: 1px solid rgba(99,102,241,0.5)"></span>
						<span class="text-muted-foreground">AI-generated (session <a href="/orgs/{slug}/traces/sessions/{s.session_id}" class="font-mono text-indigo-400 hover:underline">{s.session_short_id}</a>)</span>
					</div>
				{/each}
				<div class="flex items-center gap-2">
					<span class="inline-block h-3 w-3 rounded" style="background: rgba(128,128,128,0.15); border: 1px solid rgba(128,128,128,0.3)"></span>
					<span class="text-muted-foreground">Human / pre-existing</span>
				</div>
			</div>
		{/if}
	{:else if selectedCommitId && selectedFilePath && !attributionLoading}
		<p class="text-muted-foreground py-8 text-center text-sm">No attribution data available for this file.</p>
	{:else if !selectedCommitId}
		<p class="text-muted-foreground py-8 text-center text-sm">Select a commit to view code attribution.</p>
	{:else if !selectedFilePath}
		<p class="text-muted-foreground py-8 text-center text-sm">Select a file to view line-by-line attribution.</p>
	{/if}
</div>

<style>
	.code-blame {
		color: #e6edf3;
		background: #0d1117;
	}

	.code-blame .line-number {
		color: #484f58;
	}

	.code-blame .bg-ai-line {
		background: rgba(99, 102, 241, 0.08);
	}

	.code-blame .attribution-session {
		color: #a78bfa;
	}

	.code-blame .attribution-confidence {
		color: #6b7280;
		font-size: 10px;
	}

	.code-blame .attribution-author {
		color: #6b7280;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		display: inline-block;
		max-width: 100px;
	}

	/* hljs token overrides matching CodeView.svelte */
	.code-blame :global(.hljs-keyword),
	.code-blame :global(.hljs-selector-tag),
	.code-blame :global(.hljs-literal),
	.code-blame :global(.hljs-section),
	.code-blame :global(.hljs-link) {
		color: #ff7b72;
	}
	.code-blame :global(.hljs-string),
	.code-blame :global(.hljs-addition) {
		color: #a5d6ff;
	}
	.code-blame :global(.hljs-title),
	.code-blame :global(.hljs-title.class_),
	.code-blame :global(.hljs-title.function_) {
		color: #d2a8ff;
	}
	.code-blame :global(.hljs-type),
	.code-blame :global(.hljs-built_in),
	.code-blame :global(.hljs-builtin-name),
	.code-blame :global(.hljs-selector-id),
	.code-blame :global(.hljs-selector-attr),
	.code-blame :global(.hljs-selector-pseudo),
	.code-blame :global(.hljs-params) {
		color: #ffa657;
	}
	.code-blame :global(.hljs-number),
	.code-blame :global(.hljs-symbol) {
		color: #79c0ff;
	}
	.code-blame :global(.hljs-comment),
	.code-blame :global(.hljs-quote),
	.code-blame :global(.hljs-deletion) {
		color: #8b949e;
	}
	.code-blame :global(.hljs-meta),
	.code-blame :global(.hljs-attr) {
		color: #79c0ff;
	}
	.code-blame :global(.hljs-variable),
	.code-blame :global(.hljs-template-variable) {
		color: #ffa657;
	}
	.code-blame :global(.hljs-punctuation) {
		color: #e6edf3;
	}
</style>
