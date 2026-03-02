<script lang="ts">
	import { page } from '$app/stores';
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Table from '$lib/components/ui/table/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import * as Select from '$lib/components/ui/select/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';

	interface CommitListItem {
		id: string;
		repo_id: string;
		commit_sha: string;
		branch: string | null;
		author: string;
		session_count: number;
		total_tokens: number | null;
		created_at: string;
	}

	interface Policy {
		id: string;
		org_id: string;
		repo_id: string | null;
		name: string;
		description: string;
		condition: Record<string, unknown>;
		action: string;
		severity: string;
		enabled: boolean;
		created_at: string;
		updated_at: string;
	}

	let commits: CommitListItem[] = $state([]);
	let policies: Policy[] = $state([]);
	let repoName = $state('');
	let loading = $state(true);
	let policiesLoading = $state(true);
	let error = $state('');
	let policiesError = $state('');

	const repoId = $derived($page.params.id);

	// Create policy dialog state
	let createOpen = $state(false);
	let createLoading = $state(false);
	let createError = $state('');
	let newName = $state('');
	let newDescription = $state('');
	let newConditionType = $state('ConditionalToolCall');
	let newToolName = $state('');
	let newMinCount = $state('1');
	let newFilePatterns = $state('');
	let newAction = $state('block_push');
	let newSeverity = $state('medium');

	// For RequiredToolCall type
	let newToolNames = $state('');

	onMount(async () => {
		await Promise.all([loadCommits(), loadPolicies()]);
	});

	async function loadCommits() {
		try {
			const allCommits = await api.get<CommitListItem[]>('/api/v1/traces');
			commits = allCommits;
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load commits';
		} finally {
			loading = false;
		}
	}

	async function loadPolicies() {
		try {
			policies = await api.get<Policy[]>(`/api/v1/repos/${repoId}/policies`);
		} catch (err) {
			policiesError = err instanceof Error ? err.message : 'Failed to load policies';
		} finally {
			policiesLoading = false;
		}
	}

	function buildCondition(): Record<string, unknown> {
		if (newConditionType === 'RequiredToolCall') {
			return {
				type: 'RequiredToolCall',
				tool_names: newToolNames
					.split(',')
					.map((s) => s.trim())
					.filter((s) => s.length > 0)
			};
		} else {
			const condition: Record<string, unknown> = {
				type: 'ConditionalToolCall',
				tool_name: newToolName,
				min_count: parseInt(newMinCount) || 1
			};
			const patterns = newFilePatterns
				.split(',')
				.map((s) => s.trim())
				.filter((s) => s.length > 0);
			if (patterns.length > 0) {
				condition.when_files_match = patterns;
			}
			return condition;
		}
	}

	async function handleCreate(e: Event) {
		e.preventDefault();
		createLoading = true;
		createError = '';
		try {
			await api.post(`/api/v1/repos/${repoId}/policies`, {
				name: newName,
				description: newDescription || undefined,
				condition: buildCondition(),
				action: newAction,
				severity: newSeverity
			});
			createOpen = false;
			resetCreateForm();
			await loadPolicies();
		} catch (err) {
			createError = err instanceof Error ? err.message : 'Failed to create policy';
		} finally {
			createLoading = false;
		}
	}

	function resetCreateForm() {
		newName = '';
		newDescription = '';
		newConditionType = 'ConditionalToolCall';
		newToolName = '';
		newMinCount = '1';
		newFilePatterns = '';
		newToolNames = '';
		newAction = 'block_push';
		newSeverity = 'medium';
		createError = '';
	}

	async function togglePolicy(policy: Policy) {
		try {
			await api.put(`/api/v1/policies/${policy.id}`, {
				enabled: !policy.enabled
			});
			await loadPolicies();
		} catch (err) {
			policiesError = err instanceof Error ? err.message : 'Failed to update policy';
		}
	}

	async function deletePolicy(id: string) {
		if (!confirm('Delete this policy? This cannot be undone.')) return;
		try {
			await api.delete(`/api/v1/policies/${id}`);
			await loadPolicies();
		} catch (err) {
			policiesError = err instanceof Error ? err.message : 'Failed to delete policy';
		}
	}

	function conditionSummary(condition: Record<string, unknown>): string {
		const type = condition.type as string;
		if (type === 'RequiredToolCall') {
			const tools = (condition.tool_names as string[]) || [];
			return `Require: ${tools.join(', ')}`;
		} else if (type === 'ConditionalToolCall') {
			const tool = condition.tool_name as string;
			const min = (condition.min_count as number) || 1;
			const patterns = condition.when_files_match as string[] | undefined;
			let s = `${tool} >= ${min}x`;
			if (patterns && patterns.length > 0) {
				s += ` when ${patterns.join(', ')}`;
			}
			return s;
		}
		return JSON.stringify(condition);
	}

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleDateString();
	}

	function fmtTokens(n: number | null): string {
		if (n == null || n === 0) return '-';
		if (n >= 1000) return `${(n / 1000).toFixed(1)}k`;
		return String(n);
	}
</script>

<svelte:head>
	<title>Repo Detail - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	<div class="flex items-center gap-2">
		<a href="/repos" class="text-muted-foreground hover:underline">Repos</a>
		<span class="text-muted-foreground">/</span>
		<h1 class="text-2xl font-bold">{repoName || repoId}</h1>
	</div>

	<!-- Policies Section -->
	<Card.Root>
		<Card.Header class="flex flex-row items-center justify-between">
			<Card.Title>Policies</Card.Title>
			<Dialog.Root bind:open={createOpen} onOpenChange={(open) => { if (!open) resetCreateForm(); }}>
				<Dialog.Trigger>
					{#snippet child({ props })}
						<Button size="sm" {...props}>Add Policy</Button>
					{/snippet}
				</Dialog.Trigger>
				<Dialog.Content class="sm:max-w-lg">
					<Dialog.Header>
						<Dialog.Title>Create Policy</Dialog.Title>
						<Dialog.Description>Define a tool-call requirement for this repo.</Dialog.Description>
					</Dialog.Header>
					<form onsubmit={handleCreate} class="grid gap-4">
						{#if createError}
							<p class="text-sm text-destructive">{createError}</p>
						{/if}
						<div class="grid gap-2">
							<Label for="policy_name">Name</Label>
							<Input id="policy_name" bind:value={newName} required placeholder="e.g., Code review required" />
						</div>
						<div class="grid gap-2">
							<Label for="policy_desc">Description</Label>
							<Input id="policy_desc" bind:value={newDescription} placeholder="Optional description" />
						</div>
						<div class="grid gap-2">
							<Label>Condition Type</Label>
							<Select.Root type="single" value={newConditionType} onValueChange={(v) => { if (v) newConditionType = v; }}>
								<Select.Trigger>{newConditionType === 'ConditionalToolCall' ? 'Conditional Tool Call' : 'Required Tool Call'}</Select.Trigger>
								<Select.Content>
									<Select.Item value="ConditionalToolCall">Conditional Tool Call</Select.Item>
									<Select.Item value="RequiredToolCall">Required Tool Call</Select.Item>
								</Select.Content>
							</Select.Root>
						</div>

						{#if newConditionType === 'ConditionalToolCall'}
							<div class="grid gap-2">
								<Label for="tool_name">Tool Name</Label>
								<Input id="tool_name" bind:value={newToolName} required placeholder="e.g., mcp__codex-cli__review" />
							</div>
							<div class="grid gap-2">
								<Label for="min_count">Minimum Calls</Label>
								<Input id="min_count" type="number" min="1" bind:value={newMinCount} />
							</div>
							<div class="grid gap-2">
								<Label for="file_patterns">File Patterns (comma-separated, optional)</Label>
								<Input id="file_patterns" bind:value={newFilePatterns} placeholder="e.g., src/**/*.rs, lib/**/*.ts" />
							</div>
						{:else}
							<div class="grid gap-2">
								<Label for="tool_names">Tool Names (comma-separated)</Label>
								<Input id="tool_names" bind:value={newToolNames} required placeholder="e.g., mcp__codex-cli__review, Bash" />
							</div>
						{/if}

						<div class="grid grid-cols-2 gap-4">
							<div class="grid gap-2">
								<Label>Action</Label>
								<Select.Root type="single" value={newAction} onValueChange={(v) => { if (v) newAction = v; }}>
									<Select.Trigger>{newAction === 'block_push' ? 'Block Push' : 'Warn'}</Select.Trigger>
									<Select.Content>
										<Select.Item value="block_push">Block Push</Select.Item>
										<Select.Item value="warn">Warn</Select.Item>
									</Select.Content>
								</Select.Root>
							</div>
							<div class="grid gap-2">
								<Label>Severity</Label>
								<Select.Root type="single" value={newSeverity} onValueChange={(v) => { if (v) newSeverity = v; }}>
									<Select.Trigger>{newSeverity}</Select.Trigger>
									<Select.Content>
										<Select.Item value="critical">critical</Select.Item>
										<Select.Item value="high">high</Select.Item>
										<Select.Item value="medium">medium</Select.Item>
										<Select.Item value="low">low</Select.Item>
									</Select.Content>
								</Select.Root>
							</div>
						</div>

						<Dialog.Footer>
							<Button type="submit" disabled={createLoading}>
								{createLoading ? 'Creating...' : 'Create'}
							</Button>
						</Dialog.Footer>
					</form>
				</Dialog.Content>
			</Dialog.Root>
		</Card.Header>
		<Card.Content>
			{#if policiesLoading}
				<p class="text-muted-foreground">Loading...</p>
			{:else if policiesError}
				<p class="text-destructive">{policiesError}</p>
			{:else if policies.length === 0}
				<p class="text-muted-foreground">No policies configured. Add a policy to enforce tool-call requirements.</p>
			{:else}
				<Table.Root>
					<Table.Header>
						<Table.Row>
							<Table.Head>Name</Table.Head>
							<Table.Head>Condition</Table.Head>
							<Table.Head>Action</Table.Head>
							<Table.Head>Severity</Table.Head>
							<Table.Head>Scope</Table.Head>
							<Table.Head>Enabled</Table.Head>
							<Table.Head></Table.Head>
						</Table.Row>
					</Table.Header>
					<Table.Body>
						{#each policies as policy}
							<Table.Row>
								<Table.Cell class="font-medium">{policy.name}</Table.Cell>
								<Table.Cell class="font-mono text-xs max-w-xs truncate">{conditionSummary(policy.condition)}</Table.Cell>
								<Table.Cell>
									<Badge variant={policy.action === 'block_push' ? 'destructive' : 'secondary'}>
										{policy.action === 'block_push' ? 'Block' : 'Warn'}
									</Badge>
								</Table.Cell>
								<Table.Cell>
									<Badge variant="outline">{policy.severity}</Badge>
								</Table.Cell>
								<Table.Cell>
									<Badge variant="outline">
										{policy.repo_id ? 'repo' : 'org'}
									</Badge>
								</Table.Cell>
								<Table.Cell>
									<Button
										variant="ghost"
										size="sm"
										onclick={() => togglePolicy(policy)}
									>
										{policy.enabled ? 'On' : 'Off'}
									</Button>
								</Table.Cell>
								<Table.Cell>
									{#if policy.repo_id}
										<Button variant="destructive" size="sm" onclick={() => deletePolicy(policy.id)}>
											Delete
										</Button>
									{/if}
								</Table.Cell>
							</Table.Row>
						{/each}
					</Table.Body>
				</Table.Root>
			{/if}
		</Card.Content>
	</Card.Root>

	<!-- Commits Section -->
	<Card.Root>
		<Card.Header>
			<Card.Title>Commits</Card.Title>
		</Card.Header>
		<Card.Content>
			{#if loading}
				<p class="text-muted-foreground">Loading...</p>
			{:else if error}
				<p class="text-destructive">{error}</p>
			{:else if commits.length === 0}
				<p class="text-muted-foreground">No commits found for this repo.</p>
			{:else}
				<Table.Root>
					<Table.Header>
						<Table.Row>
							<Table.Head>Commit</Table.Head>
							<Table.Head>Author</Table.Head>
							<Table.Head>Branch</Table.Head>
							<Table.Head>Sessions</Table.Head>
							<Table.Head>Tokens</Table.Head>
							<Table.Head>Date</Table.Head>
						</Table.Row>
					</Table.Header>
					<Table.Body>
						{#each commits as commit}
							<Table.Row>
								<Table.Cell>
									<a href="/traces/{commit.id}" class="font-mono text-sm underline">
										{commit.commit_sha.slice(0, 8)}
									</a>
								</Table.Cell>
								<Table.Cell>{commit.author}</Table.Cell>
								<Table.Cell>
									{#if commit.branch}
										<Badge variant="outline">{commit.branch}</Badge>
									{:else}
										<span class="text-muted-foreground">-</span>
									{/if}
								</Table.Cell>
								<Table.Cell>{commit.session_count}</Table.Cell>
								<Table.Cell class="font-mono text-sm">{fmtTokens(commit.total_tokens)}</Table.Cell>
								<Table.Cell>{formatDate(commit.created_at)}</Table.Cell>
							</Table.Row>
						{/each}
					</Table.Body>
				</Table.Root>
			{/if}
		</Card.Content>
	</Card.Root>
</div>
