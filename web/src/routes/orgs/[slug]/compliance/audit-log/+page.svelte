<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import { features } from '$lib/stores/features';
	import EnterpriseUpgrade from '$lib/components/enterprise-upgrade.svelte';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Table from '$lib/components/ui/table/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';

	interface AuditLogEntry {
		id: string;
		actor_id: string | null;
		action: string;
		resource_type: string;
		resource_id: string | null;
		details: Record<string, unknown> | null;
		ip_address: string | null;
		user_agent: string | null;
		created_at: string;
	}

	interface AuditLogResponse {
		entries: AuditLogEntry[];
		total: number;
		page: number;
		per_page: number;
	}

	const slug = $derived($page.params.slug);

	let entries: AuditLogEntry[] = $state([]);
	let total = $state(0);
	let currentPage = $state(1);
	let perPage = $state(50);
	let loading = $state(true);
	let error = $state('');

	// Filters
	let filterAction = $state('');
	let filterResourceType = $state('');

	let expandedId = $state('');

	onMount(async () => {
		await loadEntries();
	});

	async function loadEntries() {
		loading = true;
		error = '';
		try {
			const params = new URLSearchParams();
			params.set('page', String(currentPage));
			params.set('per_page', String(perPage));
			if (filterAction) params.set('action', filterAction);
			if (filterResourceType) params.set('resource_type', filterResourceType);

			const res = await api.get<AuditLogResponse>(
				`/api/v1/orgs/${slug}/audit-log?${params}`
			);
			entries = res.entries;
			total = res.total;
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load audit log';
		} finally {
			loading = false;
		}
	}

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleString();
	}

	const totalPages = $derived(Math.ceil(total / perPage));

	async function applyFilters() {
		currentPage = 1;
		await loadEntries();
	}

	async function goToPage(p: number) {
		currentPage = p;
		await loadEntries();
	}
</script>

<svelte:head>
	<title>Audit Log - TraceVault</title>
</svelte:head>

{#if $features.audit_trail}
<div class="space-y-6">
	<div class="flex items-center gap-2">
		<a href="/orgs/{slug}/compliance" class="text-muted-foreground hover:underline">Compliance</a>
		<span class="text-muted-foreground">/</span>
		<h1 class="text-2xl font-bold">Audit Log</h1>
	</div>

	<!-- Filters -->
	<Card.Root>
		<Card.Content class="pt-6">
			<div class="flex flex-wrap gap-4 items-end">
				<div class="grid gap-1">
					<Label class="text-xs">Action</Label>
					<Input class="w-48" placeholder="e.g., trace.create" bind:value={filterAction} />
				</div>
				<div class="grid gap-1">
					<Label class="text-xs">Resource Type</Label>
					<Input
						class="w-48"
						placeholder="e.g., commit, policy"
						bind:value={filterResourceType}
					/>
				</div>
				<Button size="sm" onclick={applyFilters}>Filter</Button>
			</div>
		</Card.Content>
	</Card.Root>

	<!-- Results -->
	<Card.Root>
		<Card.Header class="flex flex-row items-center justify-between">
			<Card.Title>{total} entries</Card.Title>
			<p class="text-sm text-muted-foreground">Page {currentPage} of {totalPages || 1}</p>
		</Card.Header>
		<Card.Content>
			{#if loading}
				<p class="text-muted-foreground">Loading...</p>
			{:else if error}
				<p class="text-destructive">{error}</p>
			{:else if entries.length === 0}
				<p class="text-muted-foreground">No audit log entries found.</p>
			{:else}
				<Table.Root>
					<Table.Header>
						<Table.Row>
							<Table.Head>Time</Table.Head>
							<Table.Head>Action</Table.Head>
							<Table.Head>Resource</Table.Head>
							<Table.Head>Resource ID</Table.Head>
							<Table.Head>Details</Table.Head>
						</Table.Row>
					</Table.Header>
					<Table.Body>
						{#each entries as entry}
							<Table.Row
								class="cursor-pointer"
								onclick={() => {
									expandedId = expandedId === entry.id ? '' : entry.id;
								}}
							>
								<Table.Cell class="text-xs whitespace-nowrap"
									>{formatDate(entry.created_at)}</Table.Cell
								>
								<Table.Cell>
									<Badge variant="outline">{entry.action}</Badge>
								</Table.Cell>
								<Table.Cell class="text-xs font-mono"
									>{entry.resource_type}</Table.Cell
								>
								<Table.Cell class="text-xs font-mono"
									>{entry.resource_id ? entry.resource_id.slice(0, 8) : '-'}</Table.Cell
								>
								<Table.Cell class="text-xs max-w-xs truncate">
									{entry.details ? JSON.stringify(entry.details) : '-'}
								</Table.Cell>
							</Table.Row>
							{#if expandedId === entry.id}
								<Table.Row>
									<Table.Cell colspan={5}>
										<pre
											class="text-xs bg-muted p-3 rounded overflow-x-auto">{JSON.stringify(
												{
													id: entry.id,
													actor_id: entry.actor_id,
													action: entry.action,
													resource_type: entry.resource_type,
													resource_id: entry.resource_id,
													details: entry.details,
													ip_address: entry.ip_address,
													user_agent: entry.user_agent,
													created_at: entry.created_at
												},
												null,
												2
											)}</pre>
									</Table.Cell>
								</Table.Row>
							{/if}
						{/each}
					</Table.Body>
				</Table.Root>

				<!-- Pagination -->
				{#if totalPages > 1}
					<div class="flex items-center justify-center gap-2 mt-4">
						<Button
							variant="outline"
							size="sm"
							disabled={currentPage <= 1}
							onclick={() => goToPage(currentPage - 1)}
						>
							Previous
						</Button>
						<span class="text-sm text-muted-foreground">
							Page {currentPage} of {totalPages}
						</span>
						<Button
							variant="outline"
							size="sm"
							disabled={currentPage >= totalPages}
							onclick={() => goToPage(currentPage + 1)}
						>
							Next
						</Button>
					</div>
				{/if}
			{/if}
		</Card.Content>
	</Card.Root>
</div>
{:else}
	<EnterpriseUpgrade feature="audit_trail" />
{/if}
