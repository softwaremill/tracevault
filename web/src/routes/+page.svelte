<script lang="ts">
	import { goto } from '$app/navigation';
	import { browser } from '$app/environment';
	import { auth } from '$lib/stores/auth';

	let authState: { isAuthenticated: boolean; loading: boolean } = $state({
		isAuthenticated: false,
		loading: true
	});
	auth.subscribe((s) => (authState = s));

	$effect(() => {
		if (!browser || authState.loading) return;
		if (authState.isAuthenticated) {
			goto('/repos');
		} else {
			goto('/auth/login');
		}
	});
</script>

<div class="flex min-h-screen items-center justify-center">
	<p class="text-muted-foreground">Redirecting...</p>
</div>
