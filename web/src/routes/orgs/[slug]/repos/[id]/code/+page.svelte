<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import type { BranchInfo, TreeEntry } from '$lib/types/code';
	import BranchSelector from '$lib/components/code/BranchSelector.svelte';
	import BreadcrumbNav from '$lib/components/code/BreadcrumbNav.svelte';
	import FileTree from '$lib/components/code/FileTree.svelte';

	const repoId = $derived($page.params.id ?? '');
	const slug = $derived($page.params.slug);
	const refFromUrl = $derived($page.url.searchParams.get('ref'));

	let branches = $state<BranchInfo[]>([]);
	let entries = $state<TreeEntry[]>([]);
	let loading = $state(true);
	let error = $state('');
	let resolvedRef = $state('');

	onMount(async () => {
		try {
			const b = await api.get<BranchInfo[]>(`/api/v1/orgs/${slug}/repos/${repoId}/code/branches`);
			branches = b;
			const defaultBranch = b.find((br) => br.is_default)?.name ?? b[0]?.name ?? 'HEAD';
			resolvedRef = refFromUrl || defaultBranch;
			const t = await api.get<TreeEntry[]>(
				`/api/v1/orgs/${slug}/repos/${repoId}/code/tree?ref=${encodeURIComponent(resolvedRef)}`
			);
			entries = t;
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load code browser';
		} finally {
			loading = false;
		}
	});

	const gitRef = $derived(resolvedRef);
</script>

<svelte:head>
	<title>Code Browser - TraceVault</title>
</svelte:head>

<div class="space-y-4">
	<div class="flex items-center gap-2">
		<a href="/orgs/{slug}/repos/{repoId}" class="text-muted-foreground hover:underline">Repo</a>
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
