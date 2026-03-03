<script lang="ts">
	import { page } from '$app/stores';
	import { onMount, onDestroy } from 'svelte';
	import { api } from '$lib/api';
	import * as Card from '$lib/components/ui/card/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';

	interface RepoSettings {
		github_url: string | null;
		clone_status: string;
		has_deploy_key: boolean;
		last_fetched_at: string | null;
	}

	interface Repo {
		id: string;
		name: string;
		github_url: string | null;
		clone_status: string;
		created_at: string;
	}

	const repoId = $derived($page.params.id);

	let settings: RepoSettings | null = $state(null);
	let repoName = $state('');
	let loading = $state(true);
	let saving = $state(false);
	let error = $state('');
	let success = $state('');

	let githubUrl = $state('');
	let deployKey = $state('');
	let pollTimer: ReturnType<typeof setInterval> | null = $state(null);

	onMount(async () => {
		await Promise.all([loadSettings(), loadRepoName()]);
	});

	onDestroy(() => {
		if (pollTimer) clearInterval(pollTimer);
	});

	async function loadRepoName() {
		try {
			const repos = await api.get<Repo[]>('/api/v1/repos');
			const repo = repos.find((r) => r.id === repoId);
			if (repo) repoName = repo.name;
		} catch {
			// non-critical
		}
	}

	async function loadSettings() {
		try {
			settings = await api.get<RepoSettings>(`/api/v1/repos/${repoId}/settings`);
			githubUrl = settings.github_url ?? '';
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load settings';
		} finally {
			loading = false;
		}
	}

	async function handleSave() {
		saving = true;
		error = '';
		success = '';
		try {
			const body: Record<string, string> = {};
			if (githubUrl) body.github_url = githubUrl;
			if (deployKey.trim()) body.deploy_key = deployKey.trim();

			settings = await api.put<RepoSettings>(`/api/v1/repos/${repoId}/settings`, body);

			deployKey = '';
			success = 'Settings saved.';

			if (settings.clone_status === 'cloning') {
				startPolling();
			}
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to save settings';
		} finally {
			saving = false;
		}
	}

	function startPolling() {
		if (pollTimer) return;
		pollTimer = setInterval(async () => {
			try {
				settings = await api.get<RepoSettings>(`/api/v1/repos/${repoId}/settings`);
				if (settings.clone_status !== 'cloning') {
					if (pollTimer) clearInterval(pollTimer);
					pollTimer = null;
				}
			} catch {
				// ignore polling errors
			}
		}, 3000);
	}

	async function handleSync() {
		saving = true;
		error = '';
		success = '';
		try {
			const result = await api.post<{ status: string }>(`/api/v1/repos/${repoId}/sync`);
			if (result.status === 'cloning') {
				if (settings) settings.clone_status = 'cloning';
				startPolling();
			} else {
				await loadSettings();
				success = 'Repository synced.';
			}
		} catch (err) {
			error = err instanceof Error ? err.message : 'Sync failed';
		} finally {
			saving = false;
		}
	}

	function formatDate(iso: string | null): string {
		if (!iso) return 'Never';
		return new Date(iso).toLocaleString();
	}
</script>

<svelte:head>
	<title>Repo Settings - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	<div class="flex items-center gap-2">
		<a href="/repos" class="text-muted-foreground hover:underline">Repos</a>
		<span class="text-muted-foreground">/</span>
		<a href="/repos/{repoId}" class="text-muted-foreground hover:underline">{repoName || repoId}</a>
		<span class="text-muted-foreground">/</span>
		<h1 class="text-2xl font-bold">Settings</h1>
	</div>

	{#if loading}
		<p class="text-muted-foreground">Loading...</p>
	{:else if settings}
		{#if error}
			<div class="rounded-md border border-destructive bg-destructive/10 p-3 text-sm text-destructive">
				{error}
			</div>
		{/if}
		{#if success}
			<div class="rounded-md border border-green-500/50 bg-green-500/10 p-3 text-sm text-green-700 dark:text-green-400">
				{success}
			</div>
		{/if}

		<!-- GitHub Connection -->
		<Card.Root>
			<Card.Header>
				<Card.Title>GitHub Connection</Card.Title>
				<Card.Description>SSH URL for cloning the repository.</Card.Description>
			</Card.Header>
			<Card.Content>
				<div class="grid gap-2">
					<Label for="github_url">Repository URL (SSH)</Label>
					<Input
						id="github_url"
						bind:value={githubUrl}
						placeholder="git@github.com:org/repo.git"
					/>
					<p class="text-xs text-muted-foreground">
						Use SSH format: <code class="rounded bg-muted px-1">git@github.com:org/repo.git</code>
					</p>
				</div>
			</Card.Content>
		</Card.Root>

		<!-- Deploy Key -->
		<Card.Root>
			<Card.Header>
				<Card.Title class="flex items-center gap-2">
					Deploy Key
					{#if settings.has_deploy_key}
						<Badge variant="secondary">Configured</Badge>
					{/if}
				</Card.Title>
				<Card.Description>
					SSH deploy key for accessing private repositories.
				</Card.Description>
			</Card.Header>
			<Card.Content class="space-y-4">
				<div class="rounded-md border bg-muted/50 p-4 text-sm space-y-3">
					<p class="font-medium">Setup instructions:</p>
					<ol class="list-decimal list-inside space-y-2 text-muted-foreground">
						<li>
							Generate an ed25519 key pair locally:
							<code class="block mt-1 rounded bg-muted px-2 py-1 text-xs font-mono">ssh-keygen -t ed25519 -f tracevault-deploy-key -N ""</code>
						</li>
						<li>
							In GitHub, go to your repo's <strong>Settings → Deploy keys → Add deploy key</strong>,
							paste the contents of <code class="rounded bg-muted px-1 text-xs">tracevault-deploy-key.pub</code>
							(read-only access is sufficient).
						</li>
						<li>
							Paste the <strong>private key</strong> contents (<code class="rounded bg-muted px-1 text-xs">tracevault-deploy-key</code> file) below.
						</li>
					</ol>
				</div>
				<div class="grid gap-2">
					<Label for="deploy_key">
						{settings.has_deploy_key ? 'Replace deploy key' : 'Private key (PEM)'}
					</Label>
					<textarea
						id="deploy_key"
						bind:value={deployKey}
						placeholder="-----BEGIN OPENSSH PRIVATE KEY-----&#10;...&#10;-----END OPENSSH PRIVATE KEY-----"
						rows="6"
						class="flex w-full rounded-md border border-input bg-transparent px-3 py-2 text-sm shadow-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-ring font-mono"
					></textarea>
				</div>
			</Card.Content>
		</Card.Root>

		<!-- Sync Status -->
		<Card.Root>
			<Card.Header>
				<Card.Title>Sync Status</Card.Title>
			</Card.Header>
			<Card.Content class="space-y-3">
				<div class="flex items-center gap-3">
					<span class="text-sm text-muted-foreground">Clone status:</span>
					{#if settings.clone_status === 'ready'}
						<Badge variant="default">Ready</Badge>
					{:else if settings.clone_status === 'cloning'}
						<Badge variant="secondary">
							<svg class="mr-1 h-3 w-3 animate-spin" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
								<path d="M12 2v4m0 12v4m-7.07-3.93l2.83-2.83m8.48-8.48l2.83-2.83M2 12h4m12 0h4m-3.93 7.07l-2.83-2.83M6.34 6.34L3.51 3.51" />
							</svg>
							Cloning...
						</Badge>
					{:else if settings.clone_status === 'error'}
						<Badge variant="destructive">Error</Badge>
					{:else}
						<Badge variant="outline">Not cloned</Badge>
					{/if}
				</div>
				<div class="flex items-center gap-3">
					<span class="text-sm text-muted-foreground">Last fetched:</span>
					<span class="text-sm">{formatDate(settings.last_fetched_at)}</span>
				</div>
				<div class="flex items-center gap-3">
					{#if settings.clone_status === 'ready'}
						<Button variant="outline" size="sm" onclick={handleSync} disabled={saving}>
							Sync Now
						</Button>
						<a
							href="/repos/{repoId}/code"
							class="inline-flex items-center gap-2 rounded-md bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground hover:bg-primary/90"
						>
							Browse Code
						</a>
					{/if}
				</div>
			</Card.Content>
		</Card.Root>

		<!-- Save -->
		<div class="flex justify-end">
			<Button onclick={handleSave} disabled={saving}>
				{saving ? 'Saving...' : 'Save Settings'}
			</Button>
		</div>
	{/if}
</div>
