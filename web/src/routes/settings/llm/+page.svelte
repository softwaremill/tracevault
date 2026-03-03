<script lang="ts">
	import { api } from '$lib/api';
	import { auth } from '$lib/stores/auth';
	import * as Card from '$lib/components/ui/card/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import * as Alert from '$lib/components/ui/alert/index.js';

	interface LlmSettings {
		provider: string | null;
		has_api_key: boolean;
		model: string | null;
		base_url: string | null;
	}

	let authState: { user: { org_id: string; role: string } | null } = $state({ user: null });
	auth.subscribe((s) => (authState = s));

	let settings: LlmSettings | null = $state(null);
	let loading = $state(true);
	let editing = $state(false);
	let saving = $state(false);
	let error = $state('');
	let success = $state('');

	let editProvider = $state('anthropic');
	let editApiKey = $state('');
	let editModel = $state('');
	let editBaseUrl = $state('');

	$effect(() => {
		if (authState.user) loadSettings();
	});

	async function loadSettings() {
		if (!authState.user) return;
		loading = true;
		error = '';
		try {
			settings = await api.get<LlmSettings>(
				`/api/v1/orgs/${authState.user.org_id}/llm-settings`
			);
			editProvider = settings.provider ?? 'anthropic';
			editModel = settings.model ?? '';
			editBaseUrl = settings.base_url ?? '';
			editApiKey = '';
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load LLM settings';
		} finally {
			loading = false;
		}
	}

	function startEditing() {
		editing = true;
		success = '';
		error = '';
	}

	function cancelEditing() {
		editing = false;
		editProvider = settings?.provider ?? 'anthropic';
		editModel = settings?.model ?? '';
		editBaseUrl = settings?.base_url ?? '';
		editApiKey = '';
	}

	async function handleSave() {
		if (!authState.user) return;
		saving = true;
		error = '';
		success = '';
		try {
			const body: Record<string, string> = {
				provider: editProvider
			};
			if (editApiKey) body.api_key = editApiKey;
			if (editModel) body.model = editModel;
			if (editBaseUrl) body.base_url = editBaseUrl;

			await api.put(`/api/v1/orgs/${authState.user.org_id}/llm-settings`, body);
			editing = false;
			success = 'LLM settings saved.';
			await loadSettings();
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to save LLM settings';
		} finally {
			saving = false;
		}
	}

	const isOwnerOrAdmin = $derived(
		authState.user?.role === 'owner' || authState.user?.role === 'admin'
	);
</script>

<svelte:head>
	<title>LLM Settings - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	<h1 class="text-2xl font-bold">Settings</h1>

	<div class="flex gap-2 text-sm">
		<a href="/settings" class="text-muted-foreground hover:underline">Organization</a>
		<a href="/settings/members" class="text-muted-foreground hover:underline">Members</a>
		<a href="/settings/api-keys" class="text-muted-foreground hover:underline">API Keys</a>
		<a href="/settings/llm" class="font-semibold underline">LLM</a>
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

	{#if loading}
		<p class="text-muted-foreground">Loading...</p>
	{:else}
		<Card.Root class="max-w-lg">
			<Card.Header>
				<Card.Title>LLM Provider</Card.Title>
				<p class="text-sm text-muted-foreground">
					Configure the AI model used for code story generation.
				</p>
			</Card.Header>
			<Card.Content class="space-y-4">
				{#if editing}
					<div class="grid gap-2">
						<Label for="llm_provider">Provider</Label>
						<select
							id="llm_provider"
							bind:value={editProvider}
							class="flex h-10 w-full rounded-md border border-input bg-background px-3 py-2 text-sm"
						>
							<option value="anthropic">Anthropic</option>
							<option value="openai">OpenAI</option>
						</select>
					</div>
					<div class="grid gap-2">
						<Label for="llm_api_key">API Key</Label>
						<Input
							id="llm_api_key"
							type="password"
							bind:value={editApiKey}
							placeholder={settings?.has_api_key
								? 'Leave blank to keep current key'
								: 'Enter API key'}
						/>
						{#if settings?.has_api_key && !editApiKey}
							<p class="text-xs text-muted-foreground">
								A key is already configured. Leave blank to keep it.
							</p>
						{/if}
					</div>
					<div class="grid gap-2">
						<Label for="llm_model">Model</Label>
						<Input
							id="llm_model"
							bind:value={editModel}
							placeholder={editProvider === 'anthropic'
								? 'claude-sonnet-4-20250514'
								: 'gpt-4o'}
						/>
						<p class="text-xs text-muted-foreground">Leave blank for default.</p>
					</div>
					<div class="grid gap-2">
						<Label for="llm_base_url">Base URL (optional)</Label>
						<Input
							id="llm_base_url"
							bind:value={editBaseUrl}
							placeholder="https://api.anthropic.com"
						/>
					</div>
					<div class="flex gap-2">
						<Button onclick={handleSave} disabled={saving}>
							{saving ? 'Saving...' : 'Save'}
						</Button>
						<Button variant="outline" onclick={cancelEditing}>Cancel</Button>
					</div>
				{:else}
					<div class="flex justify-between">
						<span class="text-muted-foreground">Provider</span>
						<span class="capitalize">{settings?.provider ?? 'Not configured'}</span>
					</div>
					<div class="flex justify-between">
						<span class="text-muted-foreground">API Key</span>
						{#if settings?.has_api_key}
							<span
								class="rounded bg-green-100 px-2 py-0.5 text-xs font-medium text-green-800 dark:bg-green-900 dark:text-green-200"
								>Configured</span
							>
						{:else}
							<span class="text-muted-foreground">Not set</span>
						{/if}
					</div>
					<div class="flex justify-between">
						<span class="text-muted-foreground">Model</span>
						<span>{settings?.model ?? 'Default'}</span>
					</div>
					{#if settings?.base_url}
						<div class="flex justify-between">
							<span class="text-muted-foreground">Base URL</span>
							<span class="truncate max-w-[200px]">{settings.base_url}</span>
						</div>
					{/if}
					{#if isOwnerOrAdmin}
						<Button variant="outline" onclick={startEditing}>Edit</Button>
					{/if}
				{/if}
			</Card.Content>
		</Card.Root>
	{/if}
</div>
