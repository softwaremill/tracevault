<script lang="ts">
	import { browser } from '$app/environment';
	import { Sun, Moon } from '@lucide/svelte';
	import { Button } from '$lib/components/ui/button/index.js';

	let isDark = $state(false);

	if (browser) {
		isDark = document.documentElement.classList.contains('dark');
	}

	function toggle() {
		isDark = !isDark;
		if (isDark) {
			document.documentElement.classList.add('dark');
			localStorage.setItem('theme', 'dark');
		} else {
			document.documentElement.classList.remove('dark');
			localStorage.setItem('theme', 'light');
		}
	}
</script>

<Button variant="ghost" size="icon" onclick={toggle} aria-label="Toggle theme">
	{#if isDark}
		<Sun class="h-5 w-5" />
	{:else}
		<Moon class="h-5 w-5" />
	{/if}
</Button>
