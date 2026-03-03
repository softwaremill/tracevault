<script lang="ts">
	import { goto } from '$app/navigation';
	import { api } from '$lib/api';
	import { auth } from '$lib/stores/auth';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import * as Alert from '$lib/components/ui/alert/index.js';

	let orgName = $state('');
	let email = $state('');
	let password = $state('');
	let name = $state('');
	let error = $state('');
	let loading = $state(false);

	async function handleSubmit(e: Event) {
		e.preventDefault();
		error = '';
		loading = true;
		try {
			const resp = await api.post<{ token: string; user_id: string; org_id: string }>(
				'/api/v1/auth/register',
				{ org_name: orgName, email, password, name: name || undefined }
			);
			auth.setToken(resp.token);
			await auth.init();
			goto('/repos');
		} catch (err) {
			error = err instanceof Error ? err.message : 'Registration failed';
		} finally {
			loading = false;
		}
	}
</script>

<svelte:head>
	<title>Register - TraceVault</title>
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
			<Card.Title class="text-2xl">Create your account</Card.Title>
			<Card.Description>Register a new organization on TraceVault.</Card.Description>
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
					<Label for="org_name">Organization name</Label>
					<Input id="org_name" bind:value={orgName} required placeholder="My Company" />
				</div>
				<div class="grid gap-2">
					<Label for="name">Your name</Label>
					<Input id="name" bind:value={name} placeholder="Jane Doe" />
				</div>
				<div class="grid gap-2">
					<Label for="email">Email</Label>
					<Input id="email" type="email" bind:value={email} required placeholder="you@example.com" />
				</div>
				<div class="grid gap-2">
					<Label for="password">Password</Label>
					<Input id="password" type="password" bind:value={password} required minlength={8} />
					<p class="text-xs text-muted-foreground">Minimum 8 characters</p>
				</div>
				<Button type="submit" class="w-full" disabled={loading}>
					{loading ? 'Creating account...' : 'Register'}
				</Button>
			</form>
		</Card.Content>
		<Card.Footer class="justify-center">
			<p class="text-sm text-muted-foreground">
				Already have an account? <a href="/auth/login" class="underline">Log in</a>
			</p>
		</Card.Footer>
	</Card.Root>
	</div>
</div>
