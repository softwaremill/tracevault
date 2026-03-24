<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { api } from '$lib/api';
	import { features } from '$lib/stores/features';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';

	interface OrgItem {
		org_id: string;
		org_name: string;
		display_name: string | null;
		role: string;
	}

	const slug = $derived($page.params.slug);

	let edition = $state('community');
	features.subscribe((f) => (edition = f.edition));

	let allOrgs: OrgItem[] = $state([]);
	let loading = $state(true);

	// Create org form
	let showCreateForm = $state(false);
	let newOrgName = $state('');
	let newDisplayName = $state('');
	let creating = $state(false);
	let createError = $state('');

	// GitHub validation
	let ghOrgStatus: 'idle' | 'checking' | 'valid' | 'invalid' = $state('idle');
	let ghOrgDisplayName = $state('');
	let ghOrgAvatar = $state('');
	let ghCheckTimeout: ReturnType<typeof setTimeout> | null = null;

	onMount(async () => {
		try {
			allOrgs = await api.get<OrgItem[]>('/api/v1/me/orgs');
		} catch (e) {
			console.error(e);
		}
		loading = false;
	});

	function onNewOrgInput() {
		const val = newOrgName.trim().toLowerCase();
		if (ghCheckTimeout) clearTimeout(ghCheckTimeout);
		ghOrgStatus = 'idle';
		ghOrgDisplayName = '';
		ghOrgAvatar = '';
		if (val.length < 2) return;
		ghOrgStatus = 'checking';
		ghCheckTimeout = setTimeout(() => checkGithubOrg(val), 500);
	}

	async function checkGithubOrg(name: string) {
		try {
			const resp = await fetch(`https://api.github.com/orgs/${encodeURIComponent(name)}`);
			if (resp.ok) {
				const data = await resp.json();
				ghOrgStatus = 'valid';
				ghOrgDisplayName = data.name || data.login;
				ghOrgAvatar = data.avatar_url || '';
				if (!newDisplayName.trim()) newDisplayName = ghOrgDisplayName;
			} else {
				ghOrgStatus = 'invalid';
			}
		} catch {
			ghOrgStatus = 'idle';
		}
	}

	async function createOrg() {
		createError = '';
		creating = true;
		try {
			const resp = await api.post<{ id: string; name: string; signing_key_seed?: string }>('/api/v1/orgs', {
				name: newOrgName.trim().toLowerCase(),
				display_name: newDisplayName.trim() || null
			});
			goto(`/orgs/${resp.name}/settings/org`);
		} catch (e: any) {
			createError = e.message || 'Failed to create organization';
		} finally {
			creating = false;
		}
	}
</script>

<svelte:head>
	<title>Organizations - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	<div class="flex items-center justify-between">
		<h1 class="text-2xl font-bold">Organizations</h1>
		{#if edition === 'enterprise' && !showCreateForm}
			<Button onclick={() => (showCreateForm = true)}>New organization</Button>
		{/if}
	</div>

	{#if loading}
		<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm"><span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"></span>Loading...</div>
	{:else}
		{#if showCreateForm}
			<div class="border-border overflow-hidden rounded-lg border">
				<div class="bg-muted/30 px-4 py-3 text-sm font-semibold">Create Organization</div>
				<div class="p-4 space-y-3">
					<p class="text-xs text-muted-foreground">Link a new GitHub organization to TraceVault.</p>
					<form onsubmit={(e) => { e.preventDefault(); createOrg(); }} class="space-y-4">
						<div class="space-y-2">
							<Label for="newOrg">GitHub organization</Label>
							<Input id="newOrg" bind:value={newOrgName} oninput={onNewOrgInput} placeholder="my-github-org" required />
							{#if ghOrgStatus === 'checking'}
								<p class="text-xs text-muted-foreground">Checking GitHub...</p>
							{:else if ghOrgStatus === 'valid'}
								<div class="flex items-center gap-2 text-xs text-green-600">
									{#if ghOrgAvatar}<img src={ghOrgAvatar} alt="" class="h-5 w-5 rounded" />{/if}
									<span>{ghOrgDisplayName}</span>
								</div>
							{:else if ghOrgStatus === 'invalid'}
								<p class="text-xs text-destructive">Organization not found on GitHub.</p>
							{/if}
						</div>
						<div class="space-y-2">
							<Label for="newDisplay">Display name (optional)</Label>
							<Input id="newDisplay" bind:value={newDisplayName} placeholder="My Organization" />
						</div>
						{#if createError}
							<p class="text-sm text-destructive">{createError}</p>
						{/if}
						<div class="flex gap-2">
							<Button type="submit" disabled={creating || ghOrgStatus === 'invalid' || ghOrgStatus === 'checking'}>
								{creating ? 'Creating...' : 'Create'}
							</Button>
							<Button variant="outline" onclick={() => (showCreateForm = false)}>Cancel</Button>
						</div>
					</form>
				</div>
			</div>
		{/if}

		<div class="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
			{#each allOrgs as org}
				{@const isCurrent = org.org_name === slug}
				<a href="/orgs/{org.org_name}/settings/org" class="block">
					<div class="border-border overflow-hidden rounded-lg border h-full transition-colors hover:border-primary/50 {isCurrent ? 'border-primary shadow-sm' : ''}">
						<div class="p-4">
							<div class="flex items-start gap-3">
								<img src="https://github.com/{org.org_name}.png?size=80" alt="" class="h-10 w-10 rounded-md" />
								<div class="flex-1 min-w-0">
									<div class="flex items-start justify-between">
										<div class="space-y-0.5">
											<p class="font-semibold">{org.display_name || org.org_name}</p>
											{#if org.display_name}
												<p class="text-xs text-muted-foreground">{org.org_name}</p>
											{/if}
										</div>
										{#if isCurrent}
											<span class="rounded-full px-2 py-0.5 text-[10px]" style="background: rgba(62,207,142,0.12); color: #3ecf8e; border: 1px solid rgba(62,207,142,0.25)">{org.role}</span>
										{:else}
											<span class="rounded-full px-2 py-0.5 text-[10px]" style="background: rgba(167,139,250,0.12); color: #a78bfa; border: 1px solid rgba(167,139,250,0.25)">{org.role}</span>
										{/if}
									</div>
									{#if isCurrent}
										<p class="text-xs text-primary mt-1">Current</p>
									{/if}
								</div>
							</div>
						</div>
					</div>
				</a>
			{/each}
		</div>
	{/if}
</div>
