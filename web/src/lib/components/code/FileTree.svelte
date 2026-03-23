<script lang="ts">
	import type { TreeEntry } from '$lib/types/code';
	import { page } from '$app/stores';
	import { api } from '$lib/api';

	let {
		repoId,
		gitRef,
		currentPath,
		entries
	}: {
		repoId: string;
		gitRef: string;
		currentPath: string;
		entries: TreeEntry[];
	} = $props();

	const slug = $derived($page.params.slug);

	let expandedDirs = $state<Record<string, TreeEntry[]>>({});
	let loadingDirs = $state<Record<string, boolean>>({});

	async function toggleDir(path: string) {
		if (expandedDirs[path]) {
			const { [path]: _, ...rest } = expandedDirs;
			expandedDirs = rest;
			return;
		}
		loadingDirs = { ...loadingDirs, [path]: true };
		try {
			const children = await api.get<TreeEntry[]>(
				`/api/v1/orgs/${slug}/repos/${repoId}/code/tree?ref=${encodeURIComponent(gitRef)}&path=${encodeURIComponent(path)}`
			);
			expandedDirs = { ...expandedDirs, [path]: children };
		} catch (e) {
			console.error('Failed to load directory:', e);
		}
		const { [path]: __, ...restLoading } = loadingDirs;
		loadingDirs = restLoading;
	}

	function fileIcon(name: string): string {
		const ext = name.split('.').pop() || '';
		const icons: Record<string, string> = {
			rs: 'Rs',
			ts: 'TS',
			tsx: 'TS',
			js: 'JS',
			jsx: 'JS',
			py: 'Py',
			go: 'Go',
			java: 'Jv',
			scala: 'Sc',
			json: '{}',
			toml: 'Tm',
			yaml: 'Ym',
			yml: 'Ym',
			md: 'Md',
			sql: 'SQ',
			svelte: 'Sv',
			html: 'Ht',
			css: 'Cs'
		};
		return icons[ext] || '--';
	}

	function formatSize(bytes: number | null): string {
		if (bytes === null) return '';
		if (bytes < 1024) return `${bytes} B`;
		if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
		return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
	}

	const refParam = $derived(encodeURIComponent(gitRef));
</script>

<div class="rounded-lg border">
	<table class="w-full text-sm">
		<tbody>
			{#each entries as entry}
				<tr class="border-b last:border-b-0 hover:bg-accent/50">
					<td class="px-3 py-2">
						{#if entry.type === 'tree'}
							<button
								onclick={() => toggleDir(entry.path)}
								class="flex items-center gap-2 text-left font-medium"
							>
								<svg
									class="h-4 w-4 text-blue-500 transition-transform {expandedDirs[entry.path]
										? 'rotate-90'
										: ''}"
									viewBox="0 0 16 16"
									fill="currentColor"
								>
									<path
										d="M6.22 3.22a.75.75 0 011.06 0l4.25 4.25a.75.75 0 010 1.06l-4.25 4.25a.75.75 0 01-1.06-1.06L9.94 8 6.22 4.28a.75.75 0 010-1.06z"
									/>
								</svg>
								<svg class="h-4 w-4 text-blue-400" viewBox="0 0 16 16" fill="currentColor">
									<path
										d="M1.75 1A1.75 1.75 0 000 2.75v10.5C0 14.216.784 15 1.75 15h12.5A1.75 1.75 0 0016 13.25v-8.5A1.75 1.75 0 0014.25 3H7.5a.25.25 0 01-.2-.1l-.9-1.2C6.07 1.26 5.55 1 5 1H1.75z"
									/>
								</svg>
								<a
									href="/orgs/{slug}/repos/{repoId}/code/{entry.path}?ref={refParam}"
									class="hover:underline"
								>
									{entry.name}
								</a>
							</button>

							{#if loadingDirs[entry.path]}
								<div class="ml-10 py-1 text-xs text-muted-foreground">Loading...</div>
							{/if}

							{#if expandedDirs[entry.path]}
								<div class="ml-6 mt-1">
									{#each expandedDirs[entry.path] as child}
										<div class="flex items-center gap-2 py-1 pl-4">
											{#if child.type === 'tree'}
												<svg
													class="h-3.5 w-3.5 text-blue-400"
													viewBox="0 0 16 16"
													fill="currentColor"
												>
													<path
														d="M1.75 1A1.75 1.75 0 000 2.75v10.5C0 14.216.784 15 1.75 15h12.5A1.75 1.75 0 0016 13.25v-8.5A1.75 1.75 0 0014.25 3H7.5a.25.25 0 01-.2-.1l-.9-1.2C6.07 1.26 5.55 1 5 1H1.75z"
													/>
												</svg>
												<a
													href="/orgs/{slug}/repos/{repoId}/code/{child.path}?ref={refParam}"
													class="text-sm hover:underline">{child.name}</a
												>
											{:else}
												<span
													class="inline-flex h-3.5 w-5 items-center justify-center text-[9px] font-bold text-muted-foreground"
													>{fileIcon(child.name)}</span
												>
												<a
													href="/orgs/{slug}/repos/{repoId}/code/{child.path}?ref={refParam}"
													class="text-sm hover:underline">{child.name}</a
												>
												<span class="ml-auto text-xs text-muted-foreground"
													>{formatSize(child.size)}</span
												>
											{/if}
										</div>
									{/each}
								</div>
							{/if}
						{:else}
							<a
								href="/orgs/{slug}/repos/{repoId}/code/{entry.path}?ref={refParam}"
								class="flex items-center gap-2"
							>
								<span class="w-4"></span>
								<span
									class="inline-flex h-4 w-5 items-center justify-center text-[9px] font-bold text-muted-foreground"
									>{fileIcon(entry.name)}</span
								>
								<span class="hover:underline">{entry.name}</span>
							</a>
						{/if}
					</td>
					{#if entry.type === 'blob'}
						<td class="px-3 py-2 text-right text-xs text-muted-foreground">
							{formatSize(entry.size)}
						</td>
					{:else}
						<td></td>
					{/if}
				</tr>
			{/each}
		</tbody>
	</table>
</div>
