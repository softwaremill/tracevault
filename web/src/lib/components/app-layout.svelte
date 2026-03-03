<script lang="ts">
	import AppSidebar from '$lib/components/app-sidebar.svelte';
	import ThemeToggle from '$lib/components/theme-toggle.svelte';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';

	let { children } = $props();
</script>

<Tooltip.Provider>
	<div class="flex min-h-screen">
		<AppSidebar />
		<div class="flex flex-1 flex-col">
			<header class="flex h-14 items-center justify-end border-b px-4">
				<ThemeToggle />
			</header>
			<main class="flex-1 p-6">
				{@render children?.()}
			</main>
		</div>
	</div>
</Tooltip.Provider>
