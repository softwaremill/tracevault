<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { auth } from '$lib/stores/auth';
	import * as Card from '$lib/components/ui/card/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import * as Alert from '$lib/components/ui/alert/index.js';

	interface Org {
		id: string;
		name: string;
		plan: string;
		created_at: string;
	}

	let authState: { user: { org_id: string; role: string } | null } = $state({ user: null });
	auth.subscribe((s) => (authState = s));

	let org: Org | null = $state(null);
	let editName = $state('');
	let editing = $state(false);
	let saving = $state(false);
	let error = $state('');
	let success = $state('');

	$effect(() => {
		if (authState.user) loadOrg();
	});

	async function loadOrg() {
		if (!authState.user) return;
		try {
			org = await api.get<Org>(`/api/v1/orgs/${authState.user.org_id}`);
			editName = org.name;
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load org';
		}
	}

	async function handleSave() {
		if (!authState.user || !org) return;
		saving = true;
		error = '';
		success = '';
		try {
			await api.put(`/api/v1/orgs/${authState.user.org_id}`, { name: editName });
			org.name = editName;
			editing = false;
			success = 'Organization updated.';
			await auth.init();
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to update org';
		} finally {
			saving = false;
		}
	}

	const isOwner = $derived(authState.user?.role === 'owner');
</script>

<svelte:head>
	<title>Settings - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	<h1 class="text-2xl font-bold">Settings</h1>

	<div class="flex gap-2 text-sm">
		<a href="/settings" class="font-semibold underline">Organization</a>
		<a href="/settings/members" class="text-muted-foreground hover:underline">Members</a>
		<a href="/settings/api-keys" class="text-muted-foreground hover:underline">API Keys</a>
		<a href="/settings/llm" class="text-muted-foreground hover:underline">LLM</a>
	</div>

	{#if error}
		<Alert.Root variant="destructive">
			<Alert.Title>Error</Alert.Title>
			<Alert.Description>{error}</Alert.Description>
		</Alert.Root>
	{/if}

	{#if success}
		<Alert.Root>
			<Alert.Title>Success</Alert.Title>
			<Alert.Description>{success}</Alert.Description>
		</Alert.Root>
	{/if}

	{#if org}
		<Card.Root class="max-w-lg">
			<Card.Header>
				<Card.Title>Organization</Card.Title>
			</Card.Header>
			<Card.Content class="space-y-4">
				{#if editing}
					<div class="grid gap-2">
						<Label for="org_name">Organization name</Label>
						<Input id="org_name" bind:value={editName} />
					</div>
					<div class="flex gap-2">
						<Button onclick={handleSave} disabled={saving}>
							{saving ? 'Saving...' : 'Save'}
						</Button>
						<Button variant="outline" onclick={() => { editing = false; editName = org?.name ?? ''; }}>
							Cancel
						</Button>
					</div>
				{:else}
					<div class="flex justify-between">
						<span class="text-muted-foreground">Name</span>
						<span>{org.name}</span>
					</div>
					<div class="flex justify-between">
						<span class="text-muted-foreground">Plan</span>
						<span class="capitalize">{org.plan}</span>
					</div>
					{#if isOwner}
						<Button variant="outline" onclick={() => (editing = true)}>Edit</Button>
					{/if}
				{/if}
			</Card.Content>
		</Card.Root>
	{/if}
</div>
