<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import SessionDetailPanel from '$lib/components/session-detail/SessionDetailPanel.svelte';

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

	function fmtTokens(n: number | undefined | null): string {
		if (n == null || n === 0) return '-';
		if (n >= 1000) return `${(n / 1000).toFixed(1)}k`;
		return String(n);
	}

	interface VerifyResponse {
		commit_id: string;
		record_hash_valid: boolean;
		signature_valid: boolean;
		chain_valid: boolean;
		sealed_at: string | null;
	}

	let commit: CommitDetail | null = $state(null);
	let loading = $state(true);
	let error = $state('');
	let expandedSessions: Set<string> = $state(new Set());
	let verification: VerifyResponse | null = $state(null);
	let verifyLoading = $state(false);

	const commitId = $derived($page.params.id ?? '');
	const slug = $derived($page.params.slug);

	async function loadVerification() {
		verifyLoading = true;
		try {
			verification = await api.get<VerifyResponse>(`/api/v1/orgs/${slug}/traces/${commitId}/verify`);
		} catch {
			// Verification not available — not an error
		} finally {
			verifyLoading = false;
		}
	}

	onMount(async () => {
		try {
			commit = await api.get<CommitDetail>(`/api/v1/orgs/${slug}/traces/${commitId}`);
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load commit';
		} finally {
			loading = false;
		}
		loadVerification();
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

<div class="space-y-5">
	<!-- Header -->
	<div class="flex items-center gap-3">
		<a href="/orgs/{slug}/traces" class="text-muted-foreground text-sm hover:text-foreground transition-colors">&larr; Commits</a>
		<span class="text-muted-foreground/40">/</span>
		<h1 class="font-mono text-xl font-semibold">{commitId.slice(0, 8)}</h1>
		{#if commit?.branch}
			<span class="rounded-full px-2.5 py-0.5 text-[11px]" style="background: rgba(62,207,142,0.12); color: #3ecf8e; border: 1px solid rgba(62,207,142,0.25)">{commit.branch}</span>
		{/if}
		{#if verification}
			{#if verification.signature_valid && verification.chain_valid}
				<span class="rounded-full px-2.5 py-0.5 text-[11px]" style="background: rgba(62,207,142,0.12); color: #3ecf8e; border: 1px solid rgba(62,207,142,0.25)">Verified</span>
			{:else if verification.sealed_at}
				<span class="rounded-full px-2.5 py-0.5 text-[11px]" style="background: rgba(240,101,101,0.12); color: #f06565; border: 1px solid rgba(240,101,101,0.25)">Verification Failed</span>
			{:else}
				<span class="text-muted-foreground/50 text-[11px]">Not Sealed</span>
			{/if}
		{/if}
	</div>

	{#if loading}
		<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm">
			<span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"></span>
			Loading...
		</div>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if commit}
		<!-- Compact info bar + stats -->
		<div class="border-border overflow-hidden rounded-lg border">
			<!-- Commit metadata row -->
			<div class="bg-muted/30 flex flex-wrap items-center gap-x-6 gap-y-1 px-4 py-3 text-sm">
				<div class="flex items-center gap-2">
					<span class="text-muted-foreground text-xs uppercase tracking-wide">SHA</span>
					<span class="font-mono text-xs">{commit.commit_sha}</span>
				</div>
				<div class="flex items-center gap-2">
					<span class="text-muted-foreground text-xs uppercase tracking-wide">Author</span>
					<span class="text-xs">{commit.author}</span>
				</div>
				<div class="flex items-center gap-2">
					<span class="text-muted-foreground text-xs uppercase tracking-wide">Date</span>
					<span class="text-xs">{formatDate(commit.created_at)}</span>
				</div>
			</div>

			<!-- Stats grid -->
			<div class="grid grid-cols-2 gap-px md:grid-cols-4 lg:grid-cols-5">
				{#if aggregatedStats}
					<div class="bg-background p-3">
						<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Total Tokens</div>
						<div class="mt-1 text-lg font-semibold">{fmtTokens(aggregatedStats.totalTokens)}</div>
						<div class="text-muted-foreground text-[11px]">{fmtTokens(aggregatedStats.totalInput)} in / {fmtTokens(aggregatedStats.totalOutput)} out</div>
					</div>
				{/if}

				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Sessions</div>
					<div class="mt-1 text-lg font-semibold">{commit.sessions.length}</div>
				</div>

				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Files Changed</div>
					<div class="mt-1 text-lg font-semibold">{diffSummary.fileCount}</div>
					<div class="text-[11px]">
						<span class="text-green-600">+{diffSummary.totalAdded}</span>
						<span class="text-red-500 ml-1">-{diffSummary.totalDeleted}</span>
					</div>
				</div>

				{#if attrData}
					<div class="bg-background p-3">
						<div class="text-muted-foreground text-[11px] uppercase tracking-wide">AI Authored</div>
						<div class="mt-1 text-lg font-semibold" style="color: #a78bfa">{attrData.summary.ai_percentage.toFixed(1)}%</div>
						<div class="text-[11px]">
							<span class="text-green-600">+{attrData.summary.total_lines_added}</span>
							<span class="text-red-500 ml-1">-{attrData.summary.total_lines_deleted}</span>
						</div>
					</div>

					<div class="bg-background p-3">
						<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Human Authored</div>
						<div class="mt-1 text-lg font-semibold" style="color: #3ecf8e">{attrData.summary.human_percentage.toFixed(1)}%</div>
					</div>
				{/if}
			</div>
		</div>

		<!-- Sessions -->
		{#if commit.sessions.length > 0}
			<div class="space-y-3">
				<h2 class="text-sm font-semibold uppercase tracking-wide text-muted-foreground">Sessions</h2>
				{#each commit.sessions as session (session.id)}
					<div class="border-border overflow-hidden rounded-lg border">
						<button
							class="hover:bg-muted/40 flex w-full items-center gap-3 px-4 py-3 text-left transition-colors"
							onclick={() => toggleSession(session.id)}
						>
							<span class="text-muted-foreground/50 text-xs">{expandedSessions.has(session.id) ? '▼' : '▶'}</span>
							<span class="font-mono text-xs">{session.session_id}</span>
							{#if session.model}
								<span class="rounded-full px-2 py-0.5 text-[10px]" style="background: rgba(167,139,250,0.12); color: #a78bfa; border: 1px solid rgba(167,139,250,0.25)">{session.model}</span>
							{/if}
							{#if session.tool}
								<span class="rounded-full px-2 py-0.5 text-[10px]" style="background: rgba(79,110,247,0.12); color: #4f6ef7; border: 1px solid rgba(79,110,247,0.25)">{session.tool}</span>
							{/if}
							<span class="text-muted-foreground ml-auto flex items-center gap-4 text-xs">
								{#if session.total_tokens}
									<span>{fmtTokens(session.total_tokens)} tokens</span>
								{/if}
								{#if session.estimated_cost_usd}
									<span style="color: #3ecf8e">${session.estimated_cost_usd.toFixed(2)}</span>
								{/if}
								<span>{new Date(session.created_at).toLocaleString()}</span>
							</span>
						</button>

						{#if expandedSessions.has(session.id)}
							<div class="border-border border-t">
								<SessionDetailPanel sessionId={session.id} />
							</div>
						{/if}
					</div>
				{/each}
			</div>
		{/if}

		<!-- Changes / Diff -->
		{#if diffFiles.length > 0}
			<div class="space-y-3">
				<div class="flex items-center gap-3">
					<h2 class="text-sm font-semibold uppercase tracking-wide text-muted-foreground">Changes</h2>
					<span class="text-muted-foreground text-xs">{diffSummary.fileCount} file{diffSummary.fileCount !== 1 ? 's' : ''}</span>
					<span class="text-xs">
						<span class="text-green-600">+{diffSummary.totalAdded}</span>
						<span class="text-red-500 ml-1">-{diffSummary.totalDeleted}</span>
					</span>
					{#if attrData}
						<span class="rounded-full px-2 py-0.5 text-[10px]" style="background: rgba(167,139,250,0.12); color: #a78bfa; border: 1px solid rgba(167,139,250,0.25)">{attrData.summary.ai_percentage.toFixed(0)}% AI</span>
					{/if}
				</div>

				{#if !attrData}
					<div class="rounded-lg border px-4 py-3 text-xs" style="background: rgba(79,110,247,0.06); border-color: rgba(79,110,247,0.2); color: rgba(79,110,247,0.8)">
						Attribution data not available. Install <a href="https://usegitai.com" class="font-medium underline" target="_blank" rel="noopener">git-ai</a> to track which lines were written by AI agents vs humans.
					</div>
				{/if}

				<div class="border-border overflow-hidden rounded-lg border">
					{#each diffFiles as file}
						<div class="border-border border-b last:border-b-0">
							<button
								class="hover:bg-muted/40 flex w-full items-center gap-2 px-4 py-2 text-left text-xs transition-colors"
								onclick={() => toggleFile(file.path)}
							>
								<span class="text-muted-foreground/50">{expandedFiles.has(file.path) ? '▼' : '▶'}</span>
								<span class="font-mono font-medium">{file.path}</span>
								{#if file.old_path}
									<span class="text-muted-foreground">(from {file.old_path})</span>
								{/if}
								<span class="ml-auto">
									<span class="text-green-600">+{fileAddedCount(file)}</span>
									<span class="text-red-500 ml-1">-{fileDeletedCount(file)}</span>
								</span>
								{#if attrData && fileAiLineCount(file.path) > 0}
									<span class="rounded-full px-1.5 py-0.5 text-[10px]" style="background: rgba(167,139,250,0.12); color: #a78bfa">AI: {fileAiLineCount(file.path)}</span>
								{:else if attrData}
									<span class="text-muted-foreground/50">Human</span>
								{/if}
							</button>
							{#if expandedFiles.has(file.path)}
								<div class="overflow-x-auto">
									{#each file.hunks as hunk}
										<div class="border-border bg-muted/30 border-y px-4 py-1 font-mono text-[11px]" style="color: rgba(79,110,247,0.6)">
											@@ -{hunk.old_start},{hunk.old_count} +{hunk.new_start},{hunk.new_count} @@
										</div>
										{#each hunk.lines as line}
											{@const isAi = line.kind === 'add' && line.new_line_number != null && isAiLine(file.path, line.new_line_number)}
											<div
												class="flex font-mono text-[11px] leading-5 {
													line.kind === 'delete'
														? 'bg-red-500/8'
														: line.kind === 'add'
															? isAi
																? 'bg-violet-500/8'
																: 'bg-green-500/8'
															: ''
												}"
											>
												<span class="border-border text-muted-foreground/30 w-10 shrink-0 select-none border-r pr-1 text-right">
													{line.old_line_number ?? ''}
												</span>
												<span class="border-border text-muted-foreground/30 w-10 shrink-0 select-none border-r pr-1 text-right">
													{line.new_line_number ?? ''}
												</span>
												<span class="w-4 shrink-0 select-none text-center {
													line.kind === 'add' ? 'text-green-600' : line.kind === 'delete' ? 'text-red-500' : 'text-muted-foreground/20'
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
				</div>
			</div>
		{/if}

		<!-- Raw data sections -->
		{#if commit.attribution || commit.sessions.some((s) => s.session_data)}
			<div class="space-y-2">
				<h2 class="text-sm font-semibold uppercase tracking-wide text-muted-foreground">Raw Data</h2>
				<div class="border-border overflow-hidden rounded-lg border">
					{#if commit.attribution}
						<details class="border-border border-b last:border-b-0">
							<summary class="hover:bg-muted/40 cursor-pointer px-4 py-2.5 text-xs transition-colors">
								<span class="text-muted-foreground">Attribution Details</span>
							</summary>
							<pre class="bg-muted/20 border-border max-h-96 overflow-auto border-t p-4 font-mono text-[11px] leading-relaxed">{formatJson(commit.attribution)}</pre>
						</details>
					{/if}
					{#each commit.sessions.filter((s) => s.session_data) as session (session.id)}
						<details class="border-border border-b last:border-b-0">
							<summary class="hover:bg-muted/40 cursor-pointer px-4 py-2.5 text-xs transition-colors">
								<span class="text-muted-foreground">Session Data</span>
								<span class="font-mono ml-1">{session.session_id}</span>
							</summary>
							<pre class="bg-muted/20 border-border max-h-96 overflow-auto border-t p-4 font-mono text-[11px] leading-relaxed">{formatJson(session.session_data)}</pre>
						</details>
					{/each}
				</div>
			</div>
		{/if}
	{/if}
</div>
