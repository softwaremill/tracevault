<script lang="ts">
	import AppSidebar from '$lib/components/app-sidebar.svelte';
	import ThemeToggle from '$lib/components/theme-toggle.svelte';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';
	import { sidebarExpanded } from '$lib/stores/sidebar';
	import { PanelLeft } from '@lucide/svelte';

	let { children } = $props();

	let expanded = $state(false);
	$effect(() => {
		const unsub = sidebarExpanded.subscribe((v) => (expanded = v));
		return unsub;
	});
</script>

<Tooltip.Provider>
	<div class="flex min-h-screen">
		<AppSidebar />
		<div class="flex flex-1 flex-col">
			<header class="flex h-14 items-center justify-between border-b px-4">
				<div>
					{#if !expanded}
						<button
							onclick={sidebarExpanded.toggle}
							class="flex h-8 w-8 items-center justify-center rounded-md text-muted-foreground hover:text-foreground hover:bg-accent"
						>
							<PanelLeft class="h-4 w-4" />
						</button>
					{/if}
				</div>
				<ThemeToggle />
			</header>
			<main class="flex-1 p-6">
				{@render children?.()}
			</main>
		</div>
	</div>
</Tooltip.Provider>
