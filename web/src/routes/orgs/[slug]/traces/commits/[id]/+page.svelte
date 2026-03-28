<script lang="ts">
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import { formatDateTime } from '$lib/utils/date';

	interface SessionAttribution {
		session_id: string;
		session_short_id: string;
		confidence: number;
		line_start: number;
		line_end: number;
	}

	interface FileAttribution {
		file_path: string;
		sessions: SessionAttribution[];
	}

	interface CommitDetail {
		commit: {
			id: string;
			commit_sha: string;
			branch: string | null;
			author: string;
			message: string | null;
			committed_at: string;
		};
		diff_data: Record<string, unknown> | null;
		attributions_by_file: FileAttribution[];
	}

	let data: CommitDetail | null = $state(null);
	let loading = $state(true);
	let error = $state('');

	const slug = $derived($page.params.slug);
	const commitId = $derived($page.params.id);

	$effect(() => {
		fetchData();
	});

	async function fetchData() {
		loading = true;
		error = '';
		try {
			data = await api.get<CommitDetail>(`/api/v1/orgs/${slug}/traces/commits/${commitId}`);
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load commit';
		} finally {
			loading = false;
		}
	}

	function diffFiles(diffData: Record<string, unknown> | null): string[] {
		if (!diffData) return [];
		if (Array.isArray(diffData)) {
			return diffData.map((f: unknown) => {
				if (typeof f === 'string') return f;
				if (f && typeof f === 'object') {
					const obj = f as Record<string, unknown>;
					return String(obj.path ?? obj.file_path ?? obj.name ?? JSON.stringify(f));
				}
				return String(f);
			});
		}
		if (typeof diffData === 'object' && diffData.files && Array.isArray(diffData.files)) {
			return (diffData.files as unknown[]).map((f: unknown) => {
				if (typeof f === 'string') return f;
				if (f && typeof f === 'object') {
					const obj = f as Record<string, unknown>;
					return String(obj.path ?? obj.file_path ?? obj.name ?? JSON.stringify(f));
				}
				return String(f);
			});
		}
		return Object.keys(diffData).filter((k) => k !== 'raw');
	}
</script>

<svelte:head>
	<title>Commit {data?.commit.commit_sha.slice(0, 8) ?? ''} - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	{#if loading}
		<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm">
			<span
				class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
			></span>
			Loading...
		</div>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if data}
		<!-- Header -->
		<div>
			<a href="/orgs/{slug}/traces/commits" class="text-muted-foreground mb-2 block text-xs hover:underline"
				>&larr; Back to commits</a
			>
			<h1 class="flex items-center gap-3 text-2xl font-bold">
				<span class="font-mono">{data.commit.commit_sha.slice(0, 8)}</span>
				{#if data.commit.branch}
					<span
						class="rounded-full px-2 py-0.5 text-xs"
						style="background: rgba(79,110,247,0.12); color: #4f6ef7; border: 1px solid rgba(79,110,247,0.25)"
						>{data.commit.branch}</span
					>
				{/if}
			</h1>
			{#if data.commit.message}
				<pre class="text-muted-foreground mt-2 whitespace-pre-wrap text-sm font-mono">{data.commit.message.trim()}</pre>
			{/if}
			<div class="text-muted-foreground mt-1 flex gap-4 text-sm">
				<span>{data.commit.author}</span>
				<span>{formatDateTime(data.commit.committed_at)}</span>
			</div>
		</div>

		<!-- Diff data / file list -->
		<div class="border-border rounded-lg border">
			<h2
				class="text-muted-foreground border-border border-b px-3 pt-3 pb-2 text-sm font-semibold uppercase tracking-wide"
			>
				Files Changed
			</h2>
			{#if diffFiles(data.diff_data).length > 0}
				<ul class="divide-border divide-y">
					{#each diffFiles(data.diff_data) as filePath}
						<li class="px-3 py-2 font-mono text-xs">{filePath}</li>
					{/each}
				</ul>
			{:else}
				<p class="text-muted-foreground px-3 py-3 text-sm">No diff data available.</p>
			{/if}
		</div>

		<!-- Attribution breakdown -->
		<div class="border-border rounded-lg border">
			<h2
				class="text-muted-foreground border-border border-b px-3 pt-3 pb-2 text-sm font-semibold uppercase tracking-wide"
			>
				Attribution Breakdown
			</h2>
			{#if data.attributions_by_file.length > 0}
				<div class="divide-border divide-y">
					{#each data.attributions_by_file as fileAttr}
						<div class="px-3 py-3">
							<div class="mb-2 flex items-center justify-between">
								<span class="font-mono text-xs font-medium">{fileAttr.file_path}</span>
								<a
									href="/orgs/{slug}/traces/attribution?commit_id={data.commit.id}&file={encodeURIComponent(fileAttr.file_path)}"
									class="text-xs text-blue-500 hover:underline"
								>
									View attribution
								</a>
							</div>
							<div class="space-y-1">
								{#each fileAttr.sessions as sa}
									<div class="flex items-center gap-3 text-xs">
										<a
											href="/orgs/{slug}/traces/sessions/{sa.session_id}"
											class="font-mono underline"
											style="color: #818cf8"
										>
											{sa.session_short_id}
										</a>
										<span
											class="rounded-full px-2 py-0.5 text-[10px]"
											style="background: rgba(16,185,129,0.12); color: #10b981; border: 1px solid rgba(16,185,129,0.25)"
										>
											{(sa.confidence * 100).toFixed(0)}%
										</span>
										<span class="text-muted-foreground font-mono">
											L{sa.line_start}–{sa.line_end}
										</span>
									</div>
								{/each}
							</div>
						</div>
					{/each}
				</div>
			{:else}
				<p class="text-muted-foreground px-3 py-3 text-sm">No attributions available.</p>
			{/if}
		</div>
	{/if}
</div>
