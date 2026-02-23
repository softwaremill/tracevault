<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import * as Table from '$lib/components/ui/table/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import * as Alert from '$lib/components/ui/alert/index.js';

	interface ApiKey {
		id: string;
		name: string;
		key_prefix: string;
		created_at: string;
	}

	let keys: ApiKey[] = $state([]);
	let loading = $state(true);
	let error = $state('');

	let createOpen = $state(false);
	let createName = $state('');
	let createError = $state('');
	let createLoading = $state(false);
	let createdKey = $state('');

	onMount(() => loadKeys());

	async function loadKeys() {
		try {
			keys = await api.get<ApiKey[]>('/api/v1/api-keys');
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load API keys';
		} finally {
			loading = false;
		}
	}

	async function handleCreate(e: Event) {
		e.preventDefault();
		createError = '';
		createLoading = true;
		try {
			const resp = await api.post<{ id: string; key: string; name: string }>(
				'/api/v1/api-keys',
				{ name: createName }
			);
			createdKey = resp.key;
			createName = '';
			await loadKeys();
		} catch (err) {
			createError = err instanceof Error ? err.message : 'Failed to create API key';
		} finally {
			createLoading = false;
		}
	}

	async function deleteKey(id: string) {
		if (!confirm('Delete this API key? This cannot be undone.')) return;
		try {
			await api.delete(`/api/v1/api-keys/${id}`);
			await loadKeys();
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to delete API key';
		}
	}

	function closeDialog() {
		createOpen = false;
		createdKey = '';
		createError = '';
		createName = '';
	}

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleDateString();
	}
</script>

<svelte:head>
	<title>API Keys - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	<h1 class="text-2xl font-bold">Settings</h1>

	<div class="flex gap-2 text-sm">
		<a href="/settings" class="text-muted-foreground hover:underline">Organization</a>
		<a href="/settings/members" class="text-muted-foreground hover:underline">Members</a>
		<a href="/settings/api-keys" class="font-semibold underline">API Keys</a>
	</div>

	{#if error}
		<Alert.Root variant="destructive">
			<Alert.Title>Error</Alert.Title>
			<Alert.Description>{error}</Alert.Description>
		</Alert.Root>
	{/if}

	<div class="flex items-center justify-between">
		<h2 class="text-lg font-semibold">API Keys</h2>
		<Dialog.Root bind:open={createOpen} onOpenChange={(open) => { if (!open) closeDialog(); }}>
			<Dialog.Trigger>
				{#snippet child({ props })}
					<Button {...props}>Create API key</Button>
				{/snippet}
			</Dialog.Trigger>
			<Dialog.Content class="sm:max-w-md">
				<Dialog.Header>
					<Dialog.Title>Create API key</Dialog.Title>
					<Dialog.Description>
						API keys are used for CLI authentication and CI/CD pipelines.
					</Dialog.Description>
				</Dialog.Header>
				{#if createdKey}
					<Alert.Root class="mb-2">
						<Alert.Title>Key created</Alert.Title>
						<Alert.Description>
							Copy this key now. It will not be shown again.
						</Alert.Description>
					</Alert.Root>
					<div class="rounded border bg-muted p-3 font-mono text-sm break-all select-all">
						{createdKey}
					</div>
					<Dialog.Footer>
						<Button onclick={closeDialog}>Done</Button>
					</Dialog.Footer>
				{:else}
					{#if createError}
						<Alert.Root variant="destructive" class="mb-2">
							<Alert.Description>{createError}</Alert.Description>
						</Alert.Root>
					{/if}
					<form onsubmit={handleCreate} class="grid gap-4">
						<div class="grid gap-2">
							<Label for="key_name">Name</Label>
							<Input id="key_name" bind:value={createName} required placeholder="e.g. CI Pipeline" />
						</div>
						<Dialog.Footer>
							<Button type="submit" disabled={createLoading}>
								{createLoading ? 'Creating...' : 'Create'}
							</Button>
						</Dialog.Footer>
					</form>
				{/if}
			</Dialog.Content>
		</Dialog.Root>
	</div>

	{#if loading}
		<p class="text-muted-foreground">Loading...</p>
	{:else if keys.length === 0}
		<p class="text-muted-foreground">No API keys yet.</p>
	{:else}
		<Table.Root>
			<Table.Header>
				<Table.Row>
					<Table.Head>Name</Table.Head>
					<Table.Head>Key</Table.Head>
					<Table.Head>Created</Table.Head>
					<Table.Head>Actions</Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each keys as key}
					<Table.Row>
						<Table.Cell>{key.name}</Table.Cell>
						<Table.Cell class="font-mono text-sm">{key.key_prefix}</Table.Cell>
						<Table.Cell>{formatDate(key.created_at)}</Table.Cell>
						<Table.Cell>
							<Button variant="destructive" size="sm" onclick={() => deleteKey(key.id)}>
								Delete
							</Button>
						</Table.Cell>
					</Table.Row>
				{/each}
			</Table.Body>
		</Table.Root>
	{/if}
</div>
