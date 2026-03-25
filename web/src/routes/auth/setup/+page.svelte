<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import { api } from '$lib/api';
	import { auth } from '$lib/stores/auth';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Card from '$lib/components/ui/card/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import * as Alert from '$lib/components/ui/alert/index.js';
	import * as Select from '$lib/components/ui/select/index.js';
	import PasswordStrength from '$lib/components/password-strength.svelte';

	interface Features {
		initialized: boolean;
	}

	interface PublicOrg {
		name: string;
		display_name: string | null;
	}

	let initialized = $state(false);
	let checkingStatus = $state(true);
	let orgs = $state<PublicOrg[]>([]);

	// Registration form
	let orgName = $state('');
	let email = $state('');
	let password = $state('');
	let name = $state('');
	let error = $state('');
	let loading = $state(false);
	let passwordStrength: PasswordStrength | null = $state(null);
	let signingKeyMode: 'auto' | 'custom' = $state('auto');
	let customSigningKey = $state('');

	// GitHub org validation
	let ghOrgStatus: 'idle' | 'checking' | 'valid' | 'invalid' = $state('idle');
	let ghOrgDisplayName = $state('');
	let ghOrgAvatar = $state('');
	let ghCheckTimeout: ReturnType<typeof setTimeout> | null = null;

	function onOrgNameInput() {
		const slug = orgName.trim().toLowerCase();
		if (ghCheckTimeout) clearTimeout(ghCheckTimeout);
		ghOrgStatus = 'idle';
		ghOrgDisplayName = '';
		ghOrgAvatar = '';
		if (slug.length < 2) return;
		ghOrgStatus = 'checking';
		ghCheckTimeout = setTimeout(() => checkGithubOrg(slug), 500);
	}

	async function checkGithubOrg(slug: string) {
		try {
			const resp = await fetch(`https://api.github.com/orgs/${encodeURIComponent(slug)}`);
			if (resp.ok) {
				const data = await resp.json();
				ghOrgStatus = 'valid';
				ghOrgDisplayName = data.name || data.login;
				ghOrgAvatar = data.avatar_url || '';
			} else {
				ghOrgStatus = 'invalid';
			}
		} catch {
			ghOrgStatus = 'idle'; // network error — don't block
		}
	}

	// Post-registration signing key display
	let showSigningKey = $state(false);
	let signingKeySeed = $state('');
	let createdOrgName = $state('');
	let keyCopied = $state(false);

	// Invitation request form
	let reqOrgName = $state('');
	let reqEmail = $state('');
	let reqName = $state('');
	let reqError = $state('');
	let reqSuccess = $state(false);
	let reqLoading = $state(false);

	onMount(async () => {
		try {
			const feat = await api.get<Features>('/api/v1/features');
			initialized = feat.initialized;
			if (initialized) {
				orgs = await api.get<PublicOrg[]>('/api/v1/orgs/public');
			}
		} catch (e) {
			console.error(e);
		}
		checkingStatus = false;
	});

	async function handleRegister(e: Event) {
		e.preventDefault();
		error = '';
		if (passwordStrength && !passwordStrength.isStrong()) {
			error = 'Please choose a stronger password.';
			return;
		}
		loading = true;
		try {
			const resp = await api.post<{ token: string; user_id: string; org_name: string; signing_key_seed?: string }>(
				'/api/v1/auth/register',
				{
					org_name: orgName.trim().toLowerCase(),
					email,
					password,
					name: name || undefined,
					signing_key_seed: signingKeyMode === 'custom' && customSigningKey.trim() ? customSigningKey.trim() : undefined
				}
			);
			auth.setToken(resp.token);
			await auth.init();
			if (resp.signing_key_seed) {
				signingKeySeed = resp.signing_key_seed;
				createdOrgName = resp.org_name;
				showSigningKey = true;
			} else {
				goto(`/orgs/${resp.org_name}/repos`);
			}
		} catch (err) {
			error = err instanceof Error ? err.message : 'Registration failed';
		} finally {
			loading = false;
		}
	}

	async function handleRequestInvitation(e: Event) {
		e.preventDefault();
		reqError = '';
		reqSuccess = false;
		reqLoading = true;
		try {
			await api.post('/api/v1/invitation-requests', {
				org_name: reqOrgName,
				email: reqEmail,
				name: reqName || undefined
			});
			reqSuccess = true;
		} catch (err) {
			reqError = err instanceof Error ? err.message : 'Request failed';
		} finally {
			reqLoading = false;
		}
	}
</script>

<svelte:head>
	<title>Setup - TraceVault</title>
</svelte:head>

{#if checkingStatus}
	<div class="flex min-h-screen items-center justify-center">
		<p class="text-muted-foreground">Loading...</p>
	</div>
{:else if showSigningKey}
	<!-- One-time signing key display -->
	<div class="flex min-h-screen items-center justify-center">
		<div class="w-full max-w-lg space-y-6 p-8">
			<div class="flex justify-center">
				<img src="/logo.png" alt="TraceVault" class="h-12 w-12 rounded-xl" />
			</div>
			<Card.Root>
			<Card.Header>
				<Card.Title class="text-2xl">Save Your Signing Key</Card.Title>
				<Card.Description>Your organization has been created. Save this signing key now — it will not be shown again.</Card.Description>
			</Card.Header>
			<Card.Content class="space-y-4">
				<Alert.Root variant="destructive">
					<Alert.Title>One-time display</Alert.Title>
					<Alert.Description>This is the only time you will see this key. It is stored encrypted on the server and cannot be retrieved later. Save it in a secure location.</Alert.Description>
				</Alert.Root>

				<div class="space-y-2">
					<Label>Signing Key (Ed25519 seed, base64)</Label>
					<div class="relative">
						<code class="block w-full rounded-md border bg-muted p-3 text-xs font-mono break-all select-all">
							{signingKeySeed}
						</code>
					</div>
					<div class="flex gap-2">
						<Button
							variant="outline"
							size="sm"
							onclick={() => {
								navigator.clipboard.writeText(signingKeySeed);
								keyCopied = true;
								setTimeout(() => (keyCopied = false), 2000);
							}}
						>
							{keyCopied ? 'Copied!' : 'Copy to clipboard'}
						</Button>
						<Button
							variant="outline"
							size="sm"
							onclick={() => {
								const blob = new Blob([signingKeySeed], { type: 'text/plain' });
								const url = URL.createObjectURL(blob);
								const a = document.createElement('a');
								a.href = url;
								a.download = `${createdOrgName}-signing-key.txt`;
								a.click();
								URL.revokeObjectURL(url);
							}}
						>
							Download as file
						</Button>
					</div>
				</div>
			</Card.Content>
			<Card.Footer>
				<Button class="w-full" onclick={() => goto(`/orgs/${createdOrgName}/repos`)}>
					I've saved my key — Continue to dashboard
				</Button>
			</Card.Footer>
		</Card.Root>
		</div>
	</div>
{:else if !initialized}
	<!-- First user: show registration form -->
	<div class="flex min-h-screen items-center justify-center">
		<div class="w-full max-w-md space-y-6">
			<div class="flex justify-center">
				<img src="/logo.png" alt="TraceVault" class="h-12 w-12 rounded-xl" />
			</div>
			<Card.Root>
			<Card.Header>
				<Card.Title class="text-2xl">Set up TraceVault</Card.Title>
				<Card.Description>Create the first admin account and organization.</Card.Description>
			</Card.Header>
			<Card.Content>
				{#if error}
					<Alert.Root class="mb-4" variant="destructive">
						<Alert.Title>Error</Alert.Title>
						<Alert.Description>{error}</Alert.Description>
					</Alert.Root>
				{/if}
				<form onsubmit={handleRegister} class="grid gap-4">
					<div class="grid gap-2">
						<Label for="org_name">GitHub organization</Label>
						<Input id="org_name" bind:value={orgName} oninput={onOrgNameInput} required placeholder="my-github-org" />
						{#if ghOrgStatus === 'checking'}
							<p class="text-xs text-muted-foreground">Checking GitHub...</p>
						{:else if ghOrgStatus === 'valid'}
							<div class="flex items-center gap-2 text-xs text-green-600">
								{#if ghOrgAvatar}
									<img src={ghOrgAvatar} alt="" class="h-5 w-5 rounded" />
								{/if}
								<span>{ghOrgDisplayName}</span>
							</div>
						{:else if ghOrgStatus === 'invalid'}
							<p class="text-xs text-destructive">Organization not found on GitHub. Check the name and try again.</p>
						{:else}
							<p class="text-xs text-muted-foreground">Must match your GitHub organization name exactly (lowercase).</p>
						{/if}
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
						<Input id="password" type="password" bind:value={password} required minlength={10} />
						<PasswordStrength {password} bind:this={passwordStrength} />
					</div>
					<div class="grid gap-2">
						<Label>Signing key</Label>
						<div class="flex gap-4">
							<label class="flex items-center gap-2 text-sm">
								<input type="radio" bind:group={signingKeyMode} value="auto" />
								Auto-generate
							</label>
							<label class="flex items-center gap-2 text-sm">
								<input type="radio" bind:group={signingKeyMode} value="custom" />
								Provide my own
							</label>
						</div>
						{#if signingKeyMode === 'custom'}
							<Input
								bind:value={customSigningKey}
								placeholder="Base64-encoded 32-byte Ed25519 seed"
								required
							/>
							<p class="text-xs text-muted-foreground">
								Must be a base64-encoded 32-byte value. Used to sign traces for integrity verification.
							</p>
						{:else}
							<p class="text-xs text-muted-foreground">
								A secure Ed25519 signing key will be generated automatically.
							</p>
						{/if}
					</div>
					<Button type="submit" class="w-full" disabled={loading || ghOrgStatus === 'invalid' || ghOrgStatus === 'checking'}>
						{loading ? 'Setting up...' : 'Create Admin Account'}
					</Button>
				</form>
			</Card.Content>
		</Card.Root>
		</div>
	</div>
{:else}
	<!-- Instance already set up: show invitation request form -->
	<div class="flex min-h-screen items-center justify-center">
		<div class="w-full max-w-md space-y-6">
			<div class="flex justify-center">
				<img src="/logo.png" alt="TraceVault" class="h-12 w-12 rounded-xl" />
			</div>
			<Card.Root>
			<Card.Header>
				<Card.Title class="text-2xl">Request Access</Card.Title>
				<Card.Description>This TraceVault instance is managed by your organization. Request an invitation to join.</Card.Description>
			</Card.Header>
			<Card.Content>
				{#if reqSuccess}
					<Alert.Root class="mb-4">
						<Alert.Title>Request Sent</Alert.Title>
						<Alert.Description>Your invitation request has been submitted. An admin will review it shortly.</Alert.Description>
					</Alert.Root>
				{:else}
					{#if reqError}
						<Alert.Root class="mb-4" variant="destructive">
							<Alert.Title>Error</Alert.Title>
							<Alert.Description>{reqError}</Alert.Description>
						</Alert.Root>
					{/if}
					<form onsubmit={handleRequestInvitation} class="grid gap-4">
						<div class="grid gap-2">
							<Label>Organization</Label>
							<Select.Root type="single" value={reqOrgName} onValueChange={(v) => reqOrgName = v}>
								<Select.Trigger class="w-full">
									<span data-slot="select-value">{reqOrgName ? (orgs.find(o => o.name === reqOrgName)?.display_name || reqOrgName) : 'Select an organization'}</span>
								</Select.Trigger>
								<Select.Content>
									{#each orgs as org}
										<Select.Item value={org.name}>{org.display_name || org.name}</Select.Item>
									{/each}
								</Select.Content>
							</Select.Root>
						</div>
						<div class="grid gap-2">
							<Label for="req_name">Your name</Label>
							<Input id="req_name" bind:value={reqName} placeholder="Jane Doe" />
						</div>
						<div class="grid gap-2">
							<Label for="req_email">Email</Label>
							<Input id="req_email" type="email" bind:value={reqEmail} required placeholder="you@example.com" />
						</div>
						<Button type="submit" class="w-full" disabled={reqLoading}>
							{reqLoading ? 'Sending...' : 'Request Invitation'}
						</Button>
					</form>
				{/if}
			</Card.Content>
			<Card.Footer class="justify-center">
				<p class="text-sm text-muted-foreground">
					Already have an account? <a href="/auth/login" class="underline">Log in</a>
				</p>
			</Card.Footer>
		</Card.Root>
		</div>
	</div>
{/if}
