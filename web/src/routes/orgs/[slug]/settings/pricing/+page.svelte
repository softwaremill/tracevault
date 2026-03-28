<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import { orgStore } from '$lib/stores/org';
	import * as Table from '$lib/components/ui/table/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import * as Alert from '$lib/components/ui/alert/index.js';
	import * as Select from '$lib/components/ui/select/index.js';
	import { formatDate } from '$lib/utils/date';

	interface PricingEntry {
		id: string;
		model: string;
		input_per_mtok: number;
		output_per_mtok: number;
		cache_read_per_mtok: number;
		cache_write_per_mtok: number;
		effective_from: string;
		effective_until: string | null;
		created_at: string;
		source: string;
	}

	interface RecalculateResult {
		affected_sessions: number;
		total_old_cost: number;
		total_new_cost: number;
	}

	interface SyncStatus {
		last_synced_at: string | null;
	}

	interface SyncResult {
		models_updated: string[];
		last_synced_at: string | null;
	}

	const slug = $derived($page.params.slug);

	let orgState: { current: { role: string } | null } = $state({ current: null });
	orgStore.subscribe((s) => (orgState = s));
	const isOwnerOrAdmin = $derived(
		orgState.current?.role === 'owner' || orgState.current?.role === 'admin'
	);

	let entries: PricingEntry[] = $state([]);
	let models: string[] = $state([]);
	let loading = $state(true);
	let error = $state('');
	let success = $state('');

	let expandedModels: Set<string> = $state(new Set());

	// Dialog state
	let dialogOpen = $state(false);
	let editingEntry: PricingEntry | null = $state(null);
	let formModel = $state('');
	let formInput = $state('');
	let formOutput = $state('');
	let formCacheRead = $state('');
	let formCacheWrite = $state('');
	let formFrom = $state('');
	let formUntil = $state('');
	let formError = $state('');
	let formLoading = $state(false);

	let recalculating: string | null = $state(null);
	let syncStatus: SyncStatus | null = $state(null);
	let syncing = $state(false);

	onMount(() => loadData());

	async function loadData() {
		loading = true;
		error = '';
		try {
			[entries, models, syncStatus] = await Promise.all([
				api.get<PricingEntry[]>(`/api/v1/orgs/${slug}/pricing`),
				api.get<string[]>(`/api/v1/orgs/${slug}/pricing/models`),
				api.get<SyncStatus>(`/api/v1/orgs/${slug}/pricing/sync/status`)
			]);
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load pricing data';
		} finally {
			loading = false;
		}
	}

	const groupedByModel = $derived.by(() => {
		const groups: Map<string, PricingEntry[]> = new Map();
		for (const entry of entries) {
			const list = groups.get(entry.model) || [];
			list.push(entry);
			groups.set(entry.model, list);
		}
		return groups;
	});

	function toggleExpand(model: string) {
		const next = new Set(expandedModels);
		if (next.has(model)) next.delete(model);
		else next.add(model);
		expandedModels = next;
	}

	function isActive(entry: PricingEntry): boolean {
		return !entry.effective_until || new Date(entry.effective_until) > new Date();
	}

	function formatPrice(val: number): string {
		return `$${val.toFixed(2)}`;
	}

	function openAdd() {
		editingEntry = null;
		formModel = '';
		formInput = '';
		formOutput = '';
		formCacheRead = '';
		formCacheWrite = '';
		formFrom = '';
		formUntil = '';
		formError = '';
		dialogOpen = true;
	}

	function openEdit(entry: PricingEntry) {
		editingEntry = entry;
		formModel = entry.model;
		formInput = String(entry.input_per_mtok);
		formOutput = String(entry.output_per_mtok);
		formCacheRead = String(entry.cache_read_per_mtok);
		formCacheWrite = String(entry.cache_write_per_mtok);
		formFrom = entry.effective_from.slice(0, 10);
		formUntil = entry.effective_until ? entry.effective_until.slice(0, 10) : '';
		formError = '';
		dialogOpen = true;
	}

	function closeDialog() {
		dialogOpen = false;
		editingEntry = null;
		formError = '';
	}

	async function handleSubmit(e: Event) {
		e.preventDefault();
		formError = '';
		formLoading = true;

		const body = {
			model: formModel,
			input_per_mtok: parseFloat(formInput),
			output_per_mtok: parseFloat(formOutput),
			cache_read_per_mtok: parseFloat(formCacheRead),
			cache_write_per_mtok: parseFloat(formCacheWrite),
			effective_from: new Date(formFrom).toISOString(),
			effective_until: formUntil ? new Date(formUntil).toISOString() : null
		};

		try {
			if (editingEntry) {
				await api.put(`/api/v1/orgs/${slug}/pricing/${editingEntry.id}`, body);
				success = `Pricing for ${formModel} updated.`;
			} else {
				await api.post(`/api/v1/orgs/${slug}/pricing`, body);
				success = `Pricing for ${formModel} created.`;
			}
			closeDialog();
			await loadData();
		} catch (err) {
			formError = err instanceof Error ? err.message : 'Failed to save pricing';
		} finally {
			formLoading = false;
		}
	}

	async function handleRecalculate(entry: PricingEntry) {
		recalculating = entry.id;
		error = '';
		success = '';
		try {
			const result = await api.post<RecalculateResult>(
				`/api/v1/orgs/${slug}/pricing/${entry.id}/recalculate`
			);
			if (result) {
				const delta = result.total_new_cost - result.total_old_cost;
				const sign = delta >= 0 ? '+' : '';
				success = `Recalculated ${result.affected_sessions} sessions (cost delta: ${sign}$${delta.toFixed(2)})`;
			}
		} catch (err) {
			error = err instanceof Error ? err.message : 'Recalculation failed';
		} finally {
			recalculating = null;
		}
	}

	async function handleSync() {
		syncing = true;
		error = '';
		success = '';
		try {
			const result = await api.post<SyncResult>(`/api/v1/orgs/${slug}/pricing/sync`);
			if (result) {
				if (result.models_updated.length > 0) {
					success = `Synced pricing for: ${result.models_updated.join(', ')}`;
				} else {
					success = 'All prices are up to date.';
				}
				await loadData();
			}
		} catch (err) {
			error = err instanceof Error ? err.message : 'Sync failed';
		} finally {
			syncing = false;
		}
	}

	function timeAgo(iso: string): string {
		const diff = Date.now() - new Date(iso).getTime();
		const hours = Math.floor(diff / 3600000);
		if (hours < 1) {
			const mins = Math.floor(diff / 60000);
			return mins <= 1 ? 'just now' : `${mins} minutes ago`;
		}
		if (hours < 24) return `${hours} hour${hours === 1 ? '' : 's'} ago`;
		const days = Math.floor(hours / 24);
		return `${days} day${days === 1 ? '' : 's'} ago`;
	}
</script>

<svelte:head>
	<title>Pricing Settings - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	<div class="flex items-center gap-2">
		<a href="/orgs/{slug}/settings" class="text-muted-foreground hover:text-foreground">Settings</a>
		<span class="text-muted-foreground">/</span>
		<h1 class="text-2xl font-bold">Model Pricing</h1>
	</div>

	<p class="text-muted-foreground text-sm">
		Manage pricing rates used to calculate session costs in analytics. Changes to pricing can be
		applied retroactively.
	</p>

	<div class="rounded-md border border-blue-500/20 bg-blue-500/5 px-4 py-3 text-sm text-blue-200">
		<div class="flex items-center justify-between">
			<span>Pricing is automatically synced from <a href="https://github.com/BerriAI/litellm" target="_blank" rel="noopener" class="underline hover:text-blue-100">LiteLLM's</a> open-source pricing database. You can override any model's pricing manually.</span>
			{#if isOwnerOrAdmin}
				<div class="flex items-center gap-3 ml-4 shrink-0">
					{#if syncStatus?.last_synced_at}
						<span class="text-xs text-muted-foreground">Last synced: {timeAgo(syncStatus.last_synced_at)}</span>
					{/if}
					<Button variant="outline" size="sm" disabled={syncing} onclick={handleSync}>
						{syncing ? 'Syncing...' : 'Sync Now'}
					</Button>
				</div>
			{:else if syncStatus?.last_synced_at}
				<span class="text-xs text-muted-foreground ml-4 shrink-0">Last synced: {timeAgo(syncStatus.last_synced_at)}</span>
			{/if}
		</div>
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

	<div class="flex items-center justify-between">
		<h2 class="text-sm font-semibold uppercase tracking-wide text-muted-foreground">
			Pricing Entries
		</h2>
		{#if isOwnerOrAdmin}
			<Button onclick={openAdd}>Add Model</Button>
		{/if}
	</div>

	{#if loading}
		<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm">
			<span
				class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
			></span>
			Loading...
		</div>
	{:else if entries.length === 0}
		<p class="text-muted-foreground text-sm">No pricing entries configured.</p>
	{:else}
		<Table.Root>
			<Table.Header>
				<Table.Row>
					<Table.Head class="text-xs w-8"></Table.Head>
					<Table.Head class="text-xs">Model</Table.Head>
					<Table.Head class="text-xs">Input/Mtok</Table.Head>
					<Table.Head class="text-xs">Output/Mtok</Table.Head>
					<Table.Head class="text-xs">From</Table.Head>
					<Table.Head class="text-xs">Status</Table.Head>
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each [...groupedByModel.entries()] as [model, group]}
					{@const current = group[0]}
					{@const history = group.slice(1)}
					{@const expanded = expandedModels.has(model)}

					<Table.Row
						class="hover:bg-muted/40 transition-colors cursor-pointer"
						onclick={() => toggleExpand(model)}
					>
						<Table.Cell class="text-xs">{expanded ? '▾' : '▸'}</Table.Cell>
						<Table.Cell class="text-xs font-semibold">{model}</Table.Cell>
						<Table.Cell class="text-xs">{formatPrice(current.input_per_mtok)}</Table.Cell>
						<Table.Cell class="text-xs">{formatPrice(current.output_per_mtok)}</Table.Cell>
						<Table.Cell class="text-xs">{formatDate(current.effective_from)}</Table.Cell>
						<Table.Cell class="text-xs">
							{#if isActive(current)}
								<span
									class="rounded-full px-2 py-0.5 text-[10px]"
									style="background: rgba(62,207,142,0.12); color: #3ecf8e; border: 1px solid rgba(62,207,142,0.25)"
									>Active</span
								>
							{:else}
								<span class="text-muted-foreground text-[10px]">Expired</span>
							{/if}
						</Table.Cell>
					</Table.Row>

					{#if expanded}
						<Table.Row class="bg-muted/20">
							<Table.Cell colspan={6}>
								<div class="px-4 py-3 space-y-4">
									<div class="flex gap-6 text-xs">
										<span
											>Cache Read/Mtok: <b>{formatPrice(current.cache_read_per_mtok)}</b></span
										>
										<span
											>Cache Write/Mtok: <b
												>{formatPrice(current.cache_write_per_mtok)}</b
											></span
										>
										{#if current.effective_until}
											<span>Until: <b>{formatDate(current.effective_until)}</b></span>
										{/if}
										<span>
											{#if current.source === 'litellm_sync'}
												<span
													class="rounded-full px-2 py-0.5 text-[10px]"
													style="background: rgba(99,102,241,0.12); color: #818cf8; border: 1px solid rgba(99,102,241,0.25)"
												>auto</span>
											{:else}
												<span
													class="rounded-full px-2 py-0.5 text-[10px]"
													style="background: rgba(234,179,8,0.12); color: #eab308; border: 1px solid rgba(234,179,8,0.25)"
												>manual override</span>
											{/if}
										</span>
									</div>

									{#if history.length > 0}
										<div>
											<span class="text-xs text-muted-foreground font-semibold"
												>Previous rates:</span
											>
											<div class="mt-1 border-l-2 border-muted pl-3 space-y-1">
												{#each history as prev}
													<div class="text-xs text-muted-foreground">
														{formatDate(prev.effective_from)}{prev.effective_until
															? ` – ${formatDate(prev.effective_until)}`
															: ''}: In {formatPrice(prev.input_per_mtok)} / Out {formatPrice(
															prev.output_per_mtok
														)}
														/ CR {formatPrice(prev.cache_read_per_mtok)} / CW {formatPrice(
															prev.cache_write_per_mtok
														)}
													</div>
												{/each}
											</div>
										</div>
									{/if}

									{#if isOwnerOrAdmin}
										<div class="flex gap-2">
											<Button
												variant="outline"
												size="sm"
												onclick={() => openEdit(current)}
											>
												Edit
											</Button>
											<Button
												variant="outline"
												size="sm"
												disabled={recalculating === current.id}
												onclick={() => handleRecalculate(current)}
											>
												{recalculating === current.id
													? 'Recalculating...'
													: 'Recalculate'}
											</Button>
										</div>
									{/if}
								</div>
							</Table.Cell>
						</Table.Row>
					{/if}
				{/each}
			</Table.Body>
		</Table.Root>
	{/if}

	<Dialog.Root
		bind:open={dialogOpen}
		onOpenChange={(open) => {
			if (!open) closeDialog();
		}}
	>
		<Dialog.Content class="sm:max-w-md">
			<Dialog.Header>
				<Dialog.Title>{editingEntry ? 'Edit Pricing' : 'Add Model Pricing'}</Dialog.Title>
				<Dialog.Description>
					{editingEntry
						? 'Update pricing rates for this model.'
						: 'Add pricing rates for a model.'}
				</Dialog.Description>
			</Dialog.Header>
			{#if formError}
				<Alert.Root variant="destructive" class="mb-2">
					<Alert.Description>{formError}</Alert.Description>
				</Alert.Root>
			{/if}
			<form onsubmit={handleSubmit} class="grid gap-4">
				<div class="grid gap-2">
					<Label for="pricing_model">Model</Label>
					{#if editingEntry}
						<Input id="pricing_model" bind:value={formModel} required />
					{:else}
						<Select.Root
							type="single"
							value={formModel}
							onValueChange={(v) => {
								if (v) formModel = v;
							}}
						>
							<Select.Trigger>{formModel || 'Select model...'}</Select.Trigger>
							<Select.Content>
								{#each models as m}
									<Select.Item value={m}>{m}</Select.Item>
								{/each}
							</Select.Content>
						</Select.Root>
					{/if}
				</div>
				<div class="grid grid-cols-2 gap-4">
					<div class="grid gap-2">
						<Label for="pricing_input">Input / Mtok ($)</Label>
						<Input
							id="pricing_input"
							type="number"
							step="0.01"
							bind:value={formInput}
							required
						/>
					</div>
					<div class="grid gap-2">
						<Label for="pricing_output">Output / Mtok ($)</Label>
						<Input
							id="pricing_output"
							type="number"
							step="0.01"
							bind:value={formOutput}
							required
						/>
					</div>
				</div>
				<div class="grid grid-cols-2 gap-4">
					<div class="grid gap-2">
						<Label for="pricing_cache_read">Cache Read / Mtok ($)</Label>
						<Input
							id="pricing_cache_read"
							type="number"
							step="0.01"
							bind:value={formCacheRead}
							required
						/>
					</div>
					<div class="grid gap-2">
						<Label for="pricing_cache_write">Cache Write / Mtok ($)</Label>
						<Input
							id="pricing_cache_write"
							type="number"
							step="0.01"
							bind:value={formCacheWrite}
							required
						/>
					</div>
				</div>
				<div class="grid grid-cols-2 gap-4">
					<div class="grid gap-2">
						<Label for="pricing_from">Effective From</Label>
						<Input id="pricing_from" type="date" bind:value={formFrom} required />
					</div>
					<div class="grid gap-2">
						<Label for="pricing_until">Effective Until</Label>
						<Input id="pricing_until" type="date" bind:value={formUntil} />
					</div>
				</div>
				<Dialog.Footer>
					<Button type="submit" disabled={formLoading}>
						{formLoading ? 'Saving...' : editingEntry ? 'Update' : 'Create'}
					</Button>
				</Dialog.Footer>
			</form>
		</Dialog.Content>
	</Dialog.Root>
</div>
