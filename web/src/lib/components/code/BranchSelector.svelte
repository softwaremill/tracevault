<script lang="ts">
	import type { BranchInfo } from '$lib/types/code';
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';

	let {
		branches,
		currentRef,
		repoId
	}: {
		branches: BranchInfo[];
		currentRef: string;
		repoId: string;
	} = $props();

	let open = $state(false);
	let search = $state('');

	const branchList = $derived(
		branches.filter((b) => b.type === 'branch' && b.name.toLowerCase().includes(search.toLowerCase()))
	);
	const tagList = $derived(
		branches.filter((b) => b.type === 'tag' && b.name.toLowerCase().includes(search.toLowerCase()))
	);

	function selectRef(name: string) {
		const currentPath = $page.url.pathname;
		goto(`${currentPath}?ref=${encodeURIComponent(name)}`);
		open = false;
		search = '';
	}
</script>

<div class="relative">
	<button
		onclick={() => (open = !open)}
		class="inline-flex items-center gap-2 rounded-md border px-3 py-1.5 text-sm font-medium hover:bg-accent"
	>
		<svg class="h-4 w-4" viewBox="0 0 16 16" fill="currentColor">
			<path
				fill-rule="evenodd"
				d="M11.75 2.5a.75.75 0 100 1.5.75.75 0 000-1.5zm-2.25.75a2.25 2.25 0 113 2.122V6A2.5 2.5 0 0110 8.5H6a1 1 0 00-1 1v1.128a2.251 2.251 0 11-1.5 0V5.372a2.25 2.25 0 111.5 0v1.836A2.492 2.492 0 016 7h4a1 1 0 001-1v-.628A2.25 2.25 0 019.5 3.25zM4.25 12a.75.75 0 100 1.5.75.75 0 000-1.5zM3.5 3.25a.75.75 0 111.5 0 .75.75 0 01-1.5 0z"
			/>
		</svg>
		{currentRef}
		<svg class="h-3 w-3" viewBox="0 0 12 12" fill="currentColor">
			<path d="M6 8.825L1.175 4 2.238 2.938 6 6.7l3.763-3.763L10.825 4z" />
		</svg>
	</button>

	{#if open}
		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="fixed inset-0 z-40" onclick={() => (open = false)} onkeydown={() => {}}></div>
		<div
			class="absolute left-0 top-full z-50 mt-1 w-64 rounded-md border bg-popover p-2 shadow-lg"
		>
			<input
				type="text"
				bind:value={search}
				placeholder="Find a branch or tag..."
				class="mb-2 w-full rounded border bg-background px-2 py-1 text-sm"
			/>
			{#if branchList.length > 0}
				<div class="mb-1 px-2 text-xs font-semibold text-muted-foreground">Branches</div>
				{#each branchList as branch}
					<button
						onclick={() => selectRef(branch.name)}
						class="flex w-full items-center gap-2 rounded px-2 py-1 text-left text-sm hover:bg-accent {branch.name === currentRef ? 'font-semibold' : ''}"
					>
						{#if branch.name === currentRef}
							<svg class="h-3 w-3" viewBox="0 0 16 16" fill="currentColor">
								<path
									fill-rule="evenodd"
									d="M13.78 4.22a.75.75 0 010 1.06l-7.25 7.25a.75.75 0 01-1.06 0L2.22 9.28a.75.75 0 011.06-1.06L6 10.94l6.72-6.72a.75.75 0 011.06 0z"
								/>
							</svg>
						{:else}
							<span class="w-3"></span>
						{/if}
						{branch.name}
						{#if branch.is_default}
							<span class="ml-auto text-xs text-muted-foreground">default</span>
						{/if}
					</button>
				{/each}
			{/if}
			{#if tagList.length > 0}
				<div class="mb-1 mt-2 px-2 text-xs font-semibold text-muted-foreground">Tags</div>
				{#each tagList as tag}
					<button
						onclick={() => selectRef(tag.name)}
						class="flex w-full items-center gap-2 rounded px-2 py-1 text-left text-sm hover:bg-accent {tag.name === currentRef ? 'font-semibold' : ''}"
					>
						{#if tag.name === currentRef}
							<svg class="h-3 w-3" viewBox="0 0 16 16" fill="currentColor">
								<path
									fill-rule="evenodd"
									d="M13.78 4.22a.75.75 0 010 1.06l-7.25 7.25a.75.75 0 01-1.06 0L2.22 9.28a.75.75 0 011.06-1.06L6 10.94l6.72-6.72a.75.75 0 011.06 0z"
								/>
							</svg>
						{:else}
							<span class="w-3"></span>
						{/if}
						{tag.name}
					</button>
				{/each}
			{/if}
			{#if branchList.length === 0 && tagList.length === 0}
				<p class="px-2 py-1 text-sm text-muted-foreground">No matches</p>
			{/if}
		</div>
	{/if}
</div>
