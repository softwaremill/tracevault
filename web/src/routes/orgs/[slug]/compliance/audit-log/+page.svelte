<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import { features } from '$lib/stores/features';
	import { formatDateTime } from '$lib/utils/date';
	import EnterpriseUpgrade from '$lib/components/enterprise-upgrade.svelte';
	import * as Table from '$lib/components/ui/table/index.js';
	import * as Select from '$lib/components/ui/select/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';

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
	let availableActions: string[] = $state([]);

	let expandedId = $state('');

	onMount(async () => {
		await Promise.all([loadEntries(), loadActions()]);
	});

	async function loadActions() {
		try {
			availableActions = await api.get<string[]>(
				`/api/v1/orgs/${slug}/audit-log/actions`
			);
		} catch {
			// keep empty
		}
	}

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

	const totalPages = $derived(Math.max(1, Math.ceil(total / perPage)));
	const showingFrom = $derived(total === 0 ? 0 : (currentPage - 1) * perPage + 1);
	const showingTo = $derived(Math.min(currentPage * perPage, total));

	function onActionChange(value: string | undefined) {
		filterAction = value ?? '';
		currentPage = 1;
		loadEntries();
	}

	function onResourceTypeChange() {
		currentPage = 1;
		loadEntries();
	}

	function setPerPage(size: number) {
		perPage = size;
		currentPage = 1;
		loadEntries();
	}

	function prevPage() {
		if (currentPage > 1) {
			currentPage--;
			loadEntries();
		}
	}

	function nextPage() {
		if (currentPage < totalPages) {
			currentPage++;
			loadEntries();
		}
	}
</script>

<svelte:head>
	<title>Audit Log - TraceVault</title>
</svelte:head>

{#if !$features.loaded}
	<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm"><span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"></span>Loading...</div>
{:else if $features.audit_trail}
<div class="space-y-6">
	<div class="flex items-center gap-2">
		<a href="/orgs/{slug}/compliance" class="text-muted-foreground hover:underline">Compliance</a>
		<span class="text-muted-foreground">/</span>
		<h1 class="text-2xl font-bold">Audit Log</h1>
	</div>

	<!-- Filters -->
	<div class="flex flex-wrap items-center gap-3 rounded-lg border bg-card p-3">
		<Select.Root type="single" value={filterAction} onValueChange={onActionChange}>
			<Select.Trigger class="w-[200px]">
				{filterAction || 'All actions'}
			</Select.Trigger>
			<Select.Content>
				<Select.Item value="">All actions</Select.Item>
				{#each availableActions as action}
					<Select.Item value={action}>{action}</Select.Item>
				{/each}
			</Select.Content>
		</Select.Root>

		<div class="grid gap-1">
			<Label class="sr-only">Resource Type</Label>
			<Input
				class="w-[200px] h-9"
				placeholder="Resource type..."
				bind:value={filterResourceType}
				onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') onResourceTypeChange(); }}
				onblur={onResourceTypeChange}
			/>
		</div>
	</div>

	<!-- Table -->
	<div class="border-border overflow-hidden rounded-lg border">
		{#if loading}
			<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm"><span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"></span>Loading...</div>
		{:else if error}
			<div class="p-4"><p class="text-destructive">{error}</p></div>
		{:else if entries.length === 0}
			<div class="text-muted-foreground py-8 text-center text-sm">No audit log entries found.</div>
		{:else}
			<Table.Root class="text-xs">
				<Table.Header>
					<Table.Row class="bg-muted/30 border-border border-b">
						<Table.Head class="text-muted-foreground text-[10px] font-semibold uppercase tracking-wider">Time</Table.Head>
						<Table.Head class="text-muted-foreground text-[10px] font-semibold uppercase tracking-wider">Action</Table.Head>
						<Table.Head class="text-muted-foreground text-[10px] font-semibold uppercase tracking-wider">Resource</Table.Head>
						<Table.Head class="text-muted-foreground text-[10px] font-semibold uppercase tracking-wider">Resource ID</Table.Head>
						<Table.Head class="text-muted-foreground text-[10px] font-semibold uppercase tracking-wider">Details</Table.Head>
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each entries as entry}
						<Table.Row
							class="cursor-pointer border-l-[3px] transition-colors {expandedId === entry.id ? 'border-l-primary bg-muted/20' : 'border-l-transparent hover:border-l-primary'}"
							onclick={() => {
								expandedId = expandedId === entry.id ? '' : entry.id;
							}}
						>
							<Table.Cell class="whitespace-nowrap"
								>{formatDateTime(entry.created_at)}</Table.Cell
							>
							<Table.Cell>
								<span class="rounded-full px-2 py-0.5 text-[10px]" style="background: rgba(79,110,247,0.12); color: #4f6ef7; border: 1px solid rgba(79,110,247,0.25)">{entry.action}</span>
							</Table.Cell>
							<Table.Cell class="font-mono"
								>{entry.resource_type}</Table.Cell
							>
							<Table.Cell class="font-mono"
								>{entry.resource_id ? entry.resource_id.slice(0, 8) : '-'}</Table.Cell
							>
							<Table.Cell class="max-w-xs truncate">
								{entry.details ? JSON.stringify(entry.details) : '-'}
							</Table.Cell>
						</Table.Row>
						{#if expandedId === entry.id}
							<Table.Row>
								<Table.Cell colspan={5} class="p-0">
									<pre
										class="text-xs bg-muted p-3 overflow-x-auto">{JSON.stringify(
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

			<!-- Pagination footer -->
			<div
				class="border-border text-muted-foreground flex items-center justify-between border-t px-3 py-2 text-xs"
			>
				<span>{showingFrom}-{showingTo} of {total}</span>
				<div class="flex items-center gap-3">
					<span>Per page:</span>
					{#each [25, 50, 100] as size}
						<button
							class="rounded px-1.5 py-0.5 transition-colors {perPage === size
								? 'bg-primary text-primary-foreground'
								: 'hover:text-foreground'}"
							onclick={() => setPerPage(size)}
						>
							{size}
						</button>
					{/each}
					<span class="text-border mx-1">|</span>
					<button
						class="hover:text-foreground disabled:opacity-30"
						disabled={currentPage <= 1}
						onclick={prevPage}
					>
						<ChevronLeftIcon class="h-4 w-4" />
					</button>
					<span>{currentPage}/{totalPages}</span>
					<button
						class="hover:text-foreground disabled:opacity-30"
						disabled={currentPage >= totalPages}
						onclick={nextPage}
					>
						<ChevronRightIcon class="h-4 w-4" />
					</button>
				</div>
			</div>
		{/if}
	</div>
</div>
{:else}
	<EnterpriseUpgrade feature="audit_trail" />
{/if}
