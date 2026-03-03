<script lang="ts">
	import { goto } from '$app/navigation';
	import { api } from '$lib/api';
	import { auth } from '$lib/stores/auth';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import * as Alert from '$lib/components/ui/alert/index.js';

	let email = $state('');
	let password = $state('');
	let error = $state('');
	let loading = $state(false);

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';
		loading = true;
		try {
			const resp = await api.post<{ token: string; user_id: string; org_id: string; org_name: string; email: string; role: string }>(
				'/api/v1/auth/login',
				{ email, password }
			);
			auth.setToken(resp.token);
			await auth.init();

			const params = new URLSearchParams(window.location.search);
			const redirect = params.get('redirect');
			goto(redirect || '/repos');
		} catch (err) {
			error = err instanceof Error ? err.message : 'Login failed';
		} finally {
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>Login - TraceVault</title>
</svelte:head>

<div class="flex min-h-screen items-center justify-center">
	<div class="w-full max-w-md space-y-6">
		<div class="flex justify-center">
			<div class="flex h-12 w-12 items-center justify-center rounded-xl bg-primary text-primary-foreground">
				<svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" class="h-7 w-7">
					<circle cx="12" cy="12" r="9" />
					<line x1="12" y1="3" x2="12" y2="21" />
					<line x1="3" y1="12" x2="21" y2="12" />
					<circle cx="12" cy="12" r="3" />
				</svg>
			</div>
		</div>
		<Card.Root>
		<Card.Header>
			<Card.Title class="text-2xl">Log in to TraceVault</Card.Title>
			<Card.Description>Enter your email and password to continue.</Card.Description>
		</Card.Header>
		<Card.Content>
			{#if error}
				<Alert.Root class="mb-4" variant="destructive">
					<Alert.Title>Error</Alert.Title>
					<Alert.Description>{error}</Alert.Description>
				</Alert.Root>
			{/if}
			<form onsubmit={handleSubmit} class="grid gap-4">
				<div class="grid gap-2">
					<Label for="email">Email</Label>
					<Input id="email" type="email" bind:value={email} required placeholder="you@example.com" />
				</div>
				<div class="grid gap-2">
					<Label for="password">Password</Label>
					<Input id="password" type="password" bind:value={password} required />
				</div>
				<Button type="submit" class="w-full" disabled={loading}>
					{loading ? 'Logging in...' : 'Log in'}
				</Button>
			</form>
		</Card.Content>
		<Card.Footer class="justify-center">
			<p class="text-sm text-muted-foreground">
				Don't have an account? <a href="/auth/register" class="underline">Register</a>
			</p>
		</Card.Footer>
	</Card.Root>
	</div>
</div>
