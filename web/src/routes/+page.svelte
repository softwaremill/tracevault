<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { browser } from '$app/environment';
	import { api } from '$lib/api';
	import { auth } from '$lib/stores/auth';

	let authState: { isAuthenticated: boolean; loading: boolean } = $state({
		isAuthenticated: false,
		loading: true
	});
	auth.subscribe((s) => (authState = s));

	let checked = $state(false);
	let initialized = $state(true);

	onMount(async () => {
		try {
			const feat = await api.get<{ initialized: boolean }>('/api/v1/features');
			initialized = feat.initialized;
		} catch {
			// assume initialized
		}
		checked = true;
	});

	$effect(() => {
		if (!browser || !checked || authState.loading) return;
		if (!initialized) {
			goto('/auth/setup');
		} else if (authState.isAuthenticated) {
			goto('/orgs');
		} else {
			goto('/auth/login');
		}
	});
</script>

<div class="flex min-h-screen items-center justify-center">
	<p class="text-muted-foreground">Redirecting...</p>
</div>
