<script lang="ts">
	import { page } from '$app/stores';

	let {
		repoId,
		path,
		gitRef
	}: {
		repoId: string;
		path: string;
		gitRef: string;
	} = $props();

	const slug = $derived($page.params.slug);
	const segments = $derived(path ? path.split('/') : []);
	const refParam = $derived(encodeURIComponent(gitRef));
</script>

<nav class="flex items-center gap-1 text-sm">
	<a href="/orgs/{slug}/repos/{repoId}/code?ref={refParam}" class="text-blue-500 hover:underline">root</a>
	{#each segments as segment, i}
		<span class="text-muted-foreground">/</span>
		{#if i < segments.length - 1}
			<a
				href="/orgs/{slug}/repos/{repoId}/code/{segments.slice(0, i + 1).join('/')}?ref={refParam}"
				class="text-blue-500 hover:underline">{segment}</a
			>
		{:else}
			<span class="font-medium">{segment}</span>
		{/if}
	{/each}
</nav>
