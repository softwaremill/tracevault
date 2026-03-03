<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import type { BranchInfo, TreeEntry, BlobResponse, StoryResponse } from '$lib/types/code';
	import BranchSelector from '$lib/components/code/BranchSelector.svelte';
	import BreadcrumbNav from '$lib/components/code/BreadcrumbNav.svelte';
	import FileTree from '$lib/components/code/FileTree.svelte';
	import CodeView from '$lib/components/code/CodeView.svelte';
	import StoryPanel from '$lib/components/code/StoryPanel.svelte';

	const repoId = $derived($page.params.id);
	const filePath = $derived($page.params.path);
	let gitRef = $derived($page.url.searchParams.get('ref') || 'main');

	let branches = $state<BranchInfo[]>([]);
	let treeEntries = $state<TreeEntry[]>([]);
	let blob = $state<BlobResponse | null>(null);
	let isDirectory = $state(true);
	let loading = $state(true);
	let error = $state('');

	// Story state
	let story = $state<StoryResponse | null>(null);
	let storyLoading = $state(false);
	let selectedLine = $state<number | null>(null);

	onMount(async () => {
		try {
			branches = await api.get<BranchInfo[]>(`/api/v1/repos/${repoId}/code/branches`);

			// Try as tree first, fall back to blob
			try {
				treeEntries = await api.get<TreeEntry[]>(
					`/api/v1/repos/${repoId}/code/tree?ref=${encodeURIComponent(gitRef)}&path=${encodeURIComponent(filePath)}`
				);
				isDirectory = true;
			} catch {
				blob = await api.get<BlobResponse>(
					`/api/v1/repos/${repoId}/code/blob?ref=${encodeURIComponent(gitRef)}&path=${encodeURIComponent(filePath)}`
				);
				isDirectory = false;
			}
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load';
		} finally {
			loading = false;
		}
	});

	async function handleLineClick(line: number) {
		selectedLine = line;
		storyLoading = true;
		story = null;
		try {
			story = await api.post<StoryResponse>(`/api/v1/repos/${repoId}/story`, {
				ref: gitRef,
				path: filePath,
				line
			});
		} catch (e) {
			console.error('Story generation failed:', e);
		}
		storyLoading = false;
	}

	async function handleRegenerate() {
		if (!selectedLine) return;
		storyLoading = true;
		story = null;
		try {
			story = await api.post<StoryResponse>(`/api/v1/repos/${repoId}/story?force=true`, {
				ref: gitRef,
				path: filePath,
				line: selectedLine
			});
		} catch (e) {
			console.error('Story regeneration failed:', e);
		}
		storyLoading = false;
	}
</script>

<svelte:head>
	<title>{filePath} - Code Browser - TraceVault</title>
</svelte:head>

<div class="space-y-4">
	<div class="flex items-center gap-2">
		<a href="/repos/{repoId}" class="text-muted-foreground hover:underline">Repo</a>
		<span class="text-muted-foreground">/</span>
		<a href="/repos/{repoId}/code?ref={encodeURIComponent(gitRef)}" class="text-muted-foreground hover:underline">Code</a>
		<span class="text-muted-foreground">/</span>
		<h1 class="text-lg font-bold truncate">{filePath}</h1>
	</div>

	<div class="flex items-center gap-4">
		<BranchSelector {branches} currentRef={gitRef} {repoId} />
		<BreadcrumbNav {repoId} path={filePath} {gitRef} />
	</div>

	{#if loading}
		<p class="text-muted-foreground">Loading...</p>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if isDirectory}
		<FileTree {repoId} {gitRef} currentPath={filePath} entries={treeEntries} />
	{:else if blob?.content}
		<div class="flex items-center justify-between rounded-t-lg border border-b-0 bg-muted/50 px-4 py-2 text-sm">
			<span class="text-muted-foreground">
				{blob.language || 'Plain text'} -- {blob.size} bytes
			</span>
			<span class="text-xs text-muted-foreground">Click a line number to generate its story</span>
		</div>
		<CodeView content={blob.content} language={blob.language} onLineClick={handleLineClick} />
	{:else if blob?.truncated}
		<p class="text-muted-foreground">File too large to display ({blob.size} bytes)</p>
	{:else}
		<p class="text-muted-foreground">Binary file, cannot display</p>
	{/if}
</div>

<StoryPanel
	{story}
	loading={storyLoading}
	onClose={() => {
		story = null;
		storyLoading = false;
		selectedLine = null;
	}}
	onRegenerate={handleRegenerate}
/>
