<script lang="ts">
	import '../app.css';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { browser } from '$app/environment';
	import { auth } from '$lib/stores/auth';
	import { features } from '$lib/stores/features';

	let { children } = $props();

	let authState: { isAuthenticated: boolean; loading: boolean } = $state({
		isAuthenticated: false,
		loading: true
	});
	auth.subscribe((s) => (authState = s));

	$effect(() => {
		auth.init();
		features.init();
	});

	$effect(() => {
		if (!browser || authState.loading) return;
		const path = $page.url.pathname;
		if (!path.startsWith('/auth') && !path.startsWith('/health') && !authState.isAuthenticated) {
			goto('/auth/login');
		}
	});
</script>

<svelte:head>
	<title>TraceVault</title>
</svelte:head>

{#if authState.loading}
	<div class="flex min-h-screen items-center justify-center">
		<p class="text-muted-foreground">Loading...</p>
	</div>
{:else}
	{@render children()}
{/if}
