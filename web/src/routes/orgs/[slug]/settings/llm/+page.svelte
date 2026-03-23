<script lang="ts">
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import { orgStore } from '$lib/stores/org';
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

	const slug = $derived($page.params.slug);

	let orgState: { current: { role: string } | null } = $state({ current: null });
	orgStore.subscribe((s) => (orgState = s));

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
		if (slug) loadSettings();
	});

	async function loadSettings() {
		loading = true;
		error = '';
		try {
			settings = await api.get<LlmSettings>(
				`/api/v1/orgs/${slug}/llm-settings`
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

			await api.put(`/api/v1/orgs/${slug}/llm-settings`, body);
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
		orgState.current?.role === 'owner' || orgState.current?.role === 'admin'
	);
</script>

<svelte:head>
	<title>LLM Settings - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	<h1 class="text-2xl font-bold">LLM Configuration</h1>
	<p class="text-muted-foreground">Configure the AI model provider used for story generation across the platform.</p>

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
