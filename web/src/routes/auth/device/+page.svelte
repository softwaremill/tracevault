<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { browser } from '$app/environment';
	import { api } from '$lib/api';
	import { auth } from '$lib/stores/auth';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Alert from '$lib/components/ui/alert/index.js';

	let token = $derived($page.url.searchParams.get('token') ?? '');
	let error = $state('');
	let success = $state(false);
	let loading = $state(false);

	let authState: { isAuthenticated: boolean; loading: boolean } = $state({ isAuthenticated: false, loading: true });
	auth.subscribe((s) => (authState = s));

	$effect(() => {
		if (!browser) return;
		if (authState.loading) return;
		if (!authState.isAuthenticated && token) {
			goto(`/auth/login?redirect=${encodeURIComponent(`/auth/device?token=${token}`)}`);
		}
	});

	async function handleApprove() {
		if (!token) return;
		error = '';
		loading = true;
		try {
			await api.post(`/api/v1/auth/device/${token}/approve`);
			success = true;
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to approve device';
		} finally {
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>Approve Device - TraceVault</title>
</svelte:head>

<div class="flex min-h-screen items-center justify-center">
	<Card.Root class="w-full max-w-md">
		<Card.Header>
			<Card.Title class="text-2xl">Approve CLI Login</Card.Title>
			<Card.Description>
				A CLI client is requesting access to your TraceVault account.
			</Card.Description>
		</Card.Header>
		<Card.Content>
			{#if !token}
				<Alert.Root variant="destructive">
					<Alert.Title>Missing token</Alert.Title>
					<Alert.Description>No device token found in the URL.</Alert.Description>
				</Alert.Root>
			{:else if success}
				<Alert.Root>
					<Alert.Title>Approved</Alert.Title>
					<Alert.Description>
						The CLI has been authorized. You can close this tab.
					</Alert.Description>
				</Alert.Root>
			{:else}
				{#if error}
					<Alert.Root class="mb-4" variant="destructive">
						<Alert.Title>Error</Alert.Title>
						<Alert.Description>{error}</Alert.Description>
					</Alert.Root>
				{/if}
				<p class="mb-4 text-sm text-muted-foreground">
					Click the button below to grant CLI access to your account.
				</p>
				<Button class="w-full" onclick={handleApprove} disabled={loading}>
					{loading ? 'Approving...' : 'Approve'}
				</Button>
			{/if}
		</Card.Content>
	</Card.Root>
</div>
