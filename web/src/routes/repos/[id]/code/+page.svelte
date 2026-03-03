<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import type { BranchInfo, TreeEntry } from '$lib/types/code';
	import BranchSelector from '$lib/components/code/BranchSelector.svelte';
	import BreadcrumbNav from '$lib/components/code/BreadcrumbNav.svelte';
	import FileTree from '$lib/components/code/FileTree.svelte';

	const repoId = $derived($page.params.id);
	let gitRef = $derived($page.url.searchParams.get('ref') || 'main');

	let branches = $state<BranchInfo[]>([]);
	let entries = $state<TreeEntry[]>([]);
	let loading = $state(true);
	let error = $state('');

	onMount(async () => {
		try {
			const [b, t] = await Promise.all([
				api.get<BranchInfo[]>(`/api/v1/repos/${repoId}/code/branches`),
				api.get<TreeEntry[]>(
					`/api/v1/repos/${repoId}/code/tree?ref=${encodeURIComponent(gitRef)}`
				)
			]);
			branches = b;
			entries = t;
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load code browser';
		} finally {
			loading = false;
		}
	});
</script>

<svelte:head>
	<title>Code Browser - TraceVault</title>
</svelte:head>

<div class="space-y-4">
	<div class="flex items-center gap-2">
		<a href="/repos/{repoId}" class="text-muted-foreground hover:underline">Repo</a>
		<span class="text-muted-foreground">/</span>
		<h1 class="text-2xl font-bold">Code</h1>
	</div>

	<div class="flex items-center gap-4">
		<BranchSelector {branches} currentRef={gitRef} {repoId} />
		<BreadcrumbNav {repoId} path="" {gitRef} />
	</div>

	{#if loading}
		<p class="text-muted-foreground">Loading...</p>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else}
		<FileTree {repoId} {gitRef} currentPath="" {entries} />
	{/if}
</div>
