<script lang="ts">
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import AppSidebar from '$lib/components/app-sidebar.svelte';

	let { children } = $props();
</script>

<Sidebar.Provider>
	<AppSidebar />
	<Sidebar.Inset>
		<header class="flex h-14 items-center border-b px-4">
			<Sidebar.Trigger />
		</header>
		<main class="flex-1 p-6">
			{@render children?.()}
		</main>
	</Sidebar.Inset>
</Sidebar.Provider>
