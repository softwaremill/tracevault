<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import * as Select from '$lib/components/ui/select/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';

	interface FilterOptions {
		orgs: { id: string; name: string }[];
		repos: { id: string; name: string }[];
		authors: string[];
	}

	let filters: FilterOptions = $state({ orgs: [], repos: [], authors: [] });
	let selectedOrg = $state('');
	let selectedRepo = $state('');
	let selectedAuthor = $state('');
	let dateFrom = $state('');
	let dateTo = $state('');
	let activePreset = $state('30d');

	onMount(async () => {
		try {
			filters = await api.get<FilterOptions>(`/api/v1/orgs/${$page.params.slug}/analytics/filters`);
		} catch {
			// filters stay empty
		}
		const params = $page.url.searchParams;
		selectedOrg = params.get('org_id') ?? '';
		selectedRepo = params.get('repo') ?? '';
		selectedAuthor = params.get('author') ?? '';
		dateFrom = params.get('from') ?? '';
		dateTo = params.get('to') ?? '';
		if (dateFrom || dateTo) activePreset = '';
		else if (!params.has('from')) applyPreset('30d');
	});

	function applyPreset(preset: string) {
		activePreset = preset;
		const now = new Date();
		let from = '';
		if (preset === '7d') {
			from = new Date(now.getTime() - 7 * 86400000).toISOString();
		} else if (preset === '30d') {
			from = new Date(now.getTime() - 30 * 86400000).toISOString();
		} else if (preset === '90d') {
			from = new Date(now.getTime() - 90 * 86400000).toISOString();
		}
		dateFrom = from;
		dateTo = '';
		updateUrl();
	}

	function updateUrl() {
		const params = new URLSearchParams();
		if (selectedOrg) params.set('org_id', selectedOrg);
		if (selectedRepo) params.set('repo', selectedRepo);
		if (selectedAuthor) params.set('author', selectedAuthor);
		if (dateFrom) params.set('from', dateFrom);
		if (dateTo) params.set('to', dateTo);
		const qs = params.toString();
		const path = $page.url.pathname;
		goto(`${path}${qs ? '?' + qs : ''}`, { replaceState: true, keepFocus: true });
	}

	function onRepoChange(value: string | undefined) {
		selectedRepo = value ?? '';
		updateUrl();
	}

	function onAuthorChange(value: string | undefined) {
		selectedAuthor = value ?? '';
		updateUrl();
	}

	function onDateFromChange(e: Event) {
		const val = (e.target as HTMLInputElement).value;
		dateFrom = val ? new Date(val).toISOString() : '';
		activePreset = '';
		updateUrl();
	}

	function onDateToChange(e: Event) {
		const val = (e.target as HTMLInputElement).value;
		dateTo = val ? new Date(val).toISOString() : '';
		activePreset = '';
		updateUrl();
	}
</script>

<div class="flex flex-wrap items-center gap-3 rounded-lg border bg-card p-3">
	<Select.Root type="single" value={selectedRepo} onValueChange={onRepoChange}>
		<Select.Trigger class="w-[160px]">
			{selectedRepo || 'All repos'}
		</Select.Trigger>
		<Select.Content>
			<Select.Item value="">All repos</Select.Item>
			{#each filters.repos as repo}
				<Select.Item value={repo.name}>{repo.name}</Select.Item>
			{/each}
		</Select.Content>
	</Select.Root>

	<Select.Root type="single" value={selectedAuthor} onValueChange={onAuthorChange}>
		<Select.Trigger class="w-[160px]">
			{selectedAuthor || 'All authors'}
		</Select.Trigger>
		<Select.Content>
			<Select.Item value="">All authors</Select.Item>
			{#each filters.authors as author}
				<Select.Item value={author}>{author}</Select.Item>
			{/each}
		</Select.Content>
	</Select.Root>

	<div class="flex items-center gap-1 ml-auto">
		{#each ['7d', '30d', '90d', 'all'] as preset}
			<Button
				variant={activePreset === preset ? 'default' : 'outline'}
				size="sm"
				onclick={() => applyPreset(preset)}
			>
				{preset === 'all' ? 'All' : preset}
			</Button>
		{/each}
	</div>

	<Input type="date" class="w-[140px]" onchange={onDateFromChange} placeholder="From" />
	<Input type="date" class="w-[140px]" onchange={onDateToChange} placeholder="To" />
</div>
