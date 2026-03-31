<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import * as Select from '$lib/components/ui/select/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Calendar } from '$lib/components/ui/calendar/index.js';
	import * as Popover from '$lib/components/ui/popover/index.js';
	import { locale } from '$lib/utils/date';
	import { cn } from '$lib/utils.js';
	import {
		DateFormatter,
		type DateValue,
		getLocalTimeZone,
		parseDate,
		today
	} from '@internationalized/date';
	import CalendarIcon from '@lucide/svelte/icons/calendar';

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

	let dateFromValue = $state<DateValue | undefined>();
	let dateToValue = $state<DateValue | undefined>();
	let fromOpen = $state(false);
	let toOpen = $state(false);

	const df = new DateFormatter(locale, { dateStyle: 'medium' });

	function isoToCalendarDate(iso: string): DateValue | undefined {
		if (!iso) return undefined;
		try {
			const d = new Date(iso);
			const yyyy = d.getFullYear();
			const mm = String(d.getMonth() + 1).padStart(2, '0');
			const dd = String(d.getDate()).padStart(2, '0');
			return parseDate(`${yyyy}-${mm}-${dd}`);
		} catch {
			return undefined;
		}
	}

	function calendarDateToIso(val: DateValue): string {
		return val.toDate(getLocalTimeZone()).toISOString();
	}

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
		dateFromValue = isoToCalendarDate(dateFrom);
		dateToValue = isoToCalendarDate(dateTo);
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
		dateFromValue = isoToCalendarDate(from);
		dateToValue = undefined;
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

	function onDateFromSelect(val: DateValue | undefined) {
		dateFromValue = val;
		dateFrom = val ? calendarDateToIso(val) : '';
		activePreset = '';
		fromOpen = false;
		updateUrl();
	}

	function onDateToSelect(val: DateValue | undefined) {
		dateToValue = val;
		dateTo = val ? calendarDateToIso(val) : '';
		activePreset = '';
		toOpen = false;
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

	<Popover.Root bind:open={fromOpen}>
		<Popover.Trigger
			class={cn(
				'inline-flex h-9 w-[160px] items-center justify-start gap-2 rounded-md border bg-background px-3 text-sm font-normal',
				!dateFromValue && 'text-muted-foreground'
			)}
		>
			<CalendarIcon class="h-4 w-4" />
			{dateFromValue ? df.format(dateFromValue.toDate(getLocalTimeZone())) : 'From'}
		</Popover.Trigger>
		<Popover.Content class="w-auto p-0">
			<Calendar
				type="single"
				value={dateFromValue}
				onValueChange={onDateFromSelect}
				maxValue={dateToValue ?? today(getLocalTimeZone())}
				{locale}
			/>
		</Popover.Content>
	</Popover.Root>

	<Popover.Root bind:open={toOpen}>
		<Popover.Trigger
			class={cn(
				'inline-flex h-9 w-[160px] items-center justify-start gap-2 rounded-md border bg-background px-3 text-sm font-normal',
				!dateToValue && 'text-muted-foreground'
			)}
		>
			<CalendarIcon class="h-4 w-4" />
			{dateToValue ? df.format(dateToValue.toDate(getLocalTimeZone())) : 'To'}
		</Popover.Trigger>
		<Popover.Content class="w-auto p-0">
			<Calendar
				type="single"
				value={dateToValue}
				onValueChange={onDateToSelect}
				minValue={dateFromValue}
				maxValue={today(getLocalTimeZone())}
				{locale}
			/>
		</Popover.Content>
	</Popover.Root>
</div>
