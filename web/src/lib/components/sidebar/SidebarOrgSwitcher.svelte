<script lang="ts">
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';

	interface OrgInfo {
		org_name: string;
		display_name: string | null;
	}

	let {
		expanded,
		slug,
		orgAll,
		orgCurrent,
		edition
	}: {
		expanded: boolean;
		slug: string;
		orgAll: OrgInfo[];
		orgCurrent: OrgInfo | null;
		edition: string;
	} = $props();

	let showOrgMenu = $state(false);
	let orgMenuRef: HTMLDivElement | undefined = $state();
</script>

<svelte:window
	onmousedown={(e) => {
		if (showOrgMenu && orgMenuRef && !orgMenuRef.contains(e.target as Node)) {
			showOrgMenu = false;
		}
	}}
/>

{#if orgAll.length > 0}
	<div class="border-b px-2 py-2">
		{#if expanded}
			<div class="relative" bind:this={orgMenuRef}>
				<button
					onclick={() => (showOrgMenu = !showOrgMenu)}
					class="flex w-full items-center justify-between rounded-md px-3 py-2 text-sm font-medium hover:bg-sidebar-accent"
				>
					<span class="flex items-center gap-2 truncate">
						<img src="https://github.com/{slug}.png?size=40" alt="" class="h-5 w-5 rounded" />
						{orgCurrent?.display_name || orgCurrent?.org_name || slug}
					</span>
					<svg
						class="h-4 w-4 shrink-0 text-muted-foreground"
						viewBox="0 0 16 16"
						fill="currentColor"
					>
						<path
							d="M4.427 7.427l3.396 3.396a.25.25 0 00.354 0l3.396-3.396A.25.25 0 0011.396 7H4.604a.25.25 0 00-.177.427z"
						/>
					</svg>
				</button>
				{#if showOrgMenu}
					<div
						class="absolute left-0 right-0 top-full z-50 mt-1 rounded-md border bg-popover p-1 shadow-md"
					>
						{#each orgAll as org}
							<a
								href="/orgs/{org.org_name}/repos"
								onclick={() => (showOrgMenu = false)}
								class="flex items-center gap-2 rounded-sm px-2 py-1.5 text-sm hover:bg-accent
									{org.org_name === slug ? 'font-semibold' : ''}"
							>
								<img
									src="https://github.com/{org.org_name}.png?size=40"
									alt=""
									class="h-5 w-5 rounded"
								/>
								{org.display_name || org.org_name}
							</a>
						{/each}
						{#if edition === 'enterprise'}
							<div class="border-t mt-1 pt-1">
								<a
									href="/orgs?create=1"
									onclick={() => (showOrgMenu = false)}
									class="flex items-center gap-2 rounded-sm px-2 py-1.5 text-sm text-muted-foreground hover:bg-accent hover:text-foreground"
								>
									<svg
										class="h-4 w-4"
										viewBox="0 0 24 24"
										fill="none"
										stroke="currentColor"
										stroke-width="2"
										><line x1="12" y1="5" x2="12" y2="19" /><line
											x1="5"
											y1="12"
											x2="19"
											y2="12"
										/></svg
									>
									Create organization
								</a>
							</div>
						{/if}
					</div>
				{/if}
			</div>
		{:else}
			<Tooltip.Root>
				<Tooltip.Trigger>
					<div class="flex h-10 w-10 items-center justify-center">
						<img src="https://github.com/{slug}.png?size=40" alt="" class="h-8 w-8 rounded" />
					</div>
				</Tooltip.Trigger>
				<Tooltip.Content side="right">
					{orgCurrent?.display_name || orgCurrent?.org_name || slug}
				</Tooltip.Content>
			</Tooltip.Root>
		{/if}
	</div>
{/if}
