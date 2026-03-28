<script lang="ts">
	import * as Table from '$lib/components/ui/table/index.js';
	import SearchIcon from '@lucide/svelte/icons/search';
	import ChevronLeftIcon from '@lucide/svelte/icons/chevron-left';
	import ChevronRightIcon from '@lucide/svelte/icons/chevron-right';
	import XIcon from '@lucide/svelte/icons/x';

	interface Column {
		key: string;
		label: string;
		sortable?: boolean;
		class?: string;
		headerClass?: string;
	}

	// eslint-disable-next-line @typescript-eslint/no-explicit-any
	type Row = Record<string, any>;

	interface Props {
		columns: Column[];
		rows: Row[];
		searchKeys?: string[];
		defaultSort?: string;
		defaultSortDir?: 'asc' | 'desc';
		defaultPageSize?: number;
		onRowClick?: (row: Row) => void;
		activeRowId?: string | null;
		rowIdKey?: string;
		children?: import('svelte').Snippet<[{ row: Row; col: Column }]>;
		expandedRow?: import('svelte').Snippet<[{ row: Row }]>;
		expandedRowId?: string | null;
	}

	let {
		columns,
		rows,
		searchKeys = [],
		defaultSort = '',
		defaultSortDir = 'desc',
		defaultPageSize = 10,
		onRowClick,
		activeRowId = null,
		rowIdKey = 'id',
		children,
		expandedRow,
		expandedRowId = null
	}: Props = $props();

	let search = $state('');
	// svelte-ignore state_referenced_locally
	let sortCol = $state(defaultSort);
	// svelte-ignore state_referenced_locally
	let sortDir = $state<'asc' | 'desc'>(defaultSortDir);
	// svelte-ignore state_referenced_locally
	let pageSize = $state(defaultPageSize);
	let currentPage = $state(0);

	function sortBy(col: string) {
		if (sortCol === col) {
			sortDir = sortDir === 'asc' ? 'desc' : 'asc';
		} else {
			sortCol = col;
			sortDir = 'desc';
		}
		currentPage = 0;
	}

	function sortIndicator(col: string): string {
		if (sortCol !== col) return '';
		return sortDir === 'asc' ? ' ↑' : ' ↓';
	}

	const filtered = $derived.by(() => {
		if (!search.trim()) return rows;
		const q = search.toLowerCase();
		return rows.filter((row) =>
			searchKeys.some((key) => {
				const val = row[key];
				return val != null && String(val).toLowerCase().includes(q);
			})
		);
	});

	const sorted = $derived.by(() => {
		if (!sortCol) return filtered;
		return [...filtered].sort((a, b) => {
			const av = a[sortCol];
			const bv = b[sortCol];
			if (av == null && bv == null) return 0;
			if (av == null) return 1;
			if (bv == null) return -1;
			const diff =
				typeof av === 'string'
					? (av as string).localeCompare(bv as string)
					: (av as number) - (bv as number);
			return sortDir === 'asc' ? diff : -diff;
		});
	});

	const totalPages = $derived(Math.max(1, Math.ceil(sorted.length / pageSize)));
	const paged = $derived(sorted.slice(currentPage * pageSize, (currentPage + 1) * pageSize));
	const showingFrom = $derived(sorted.length === 0 ? 0 : currentPage * pageSize + 1);
	const showingTo = $derived(Math.min((currentPage + 1) * pageSize, sorted.length));
</script>

<div class="border-border overflow-hidden rounded-lg border">
	<!-- Search bar -->
	{#if searchKeys.length > 0}
		<div class="border-border flex items-center gap-2 border-b px-3 py-2">
			<SearchIcon class="text-muted-foreground h-3.5 w-3.5 shrink-0" />
			<input
				type="text"
				placeholder="Search..."
				bind:value={search}
				oninput={() => (currentPage = 0)}
				class="text-foreground placeholder:text-muted-foreground w-full bg-transparent text-sm outline-none"
			/>
			{#if search}
				<button
					class="text-muted-foreground hover:text-foreground"
					onclick={() => {
						search = '';
						currentPage = 0;
					}}
				>
					<XIcon class="h-3.5 w-3.5" />
				</button>
			{/if}
		</div>
	{/if}

	<!-- Table -->
	<Table.Root class="text-xs">
		<Table.Header>
			<Table.Row class="bg-muted/30 border-border border-b">
				{#each columns as col}
					<Table.Head
						class="text-muted-foreground text-[10px] font-semibold uppercase tracking-wider {col.headerClass ??
							''}"
					>
						{#if col.sortable}
							<button
								class="hover:text-foreground transition-colors"
								onclick={() => sortBy(col.key)}
							>
								{col.label}{sortIndicator(col.key)}
							</button>
						{:else}
							{col.label}
						{/if}
					</Table.Head>
				{/each}
			</Table.Row>
		</Table.Header>
		<Table.Body>
			{#each paged as row (row[rowIdKey] ?? JSON.stringify(row))}
				<Table.Row
					class="{onRowClick ? 'cursor-pointer hover:border-l-primary' : ''} border-l-[3px] transition-colors {activeRowId !=
						null && row[rowIdKey] === activeRowId
						? 'border-l-primary bg-muted/20'
						: 'border-l-transparent'}"
					onclick={() => onRowClick?.(row)}
				>
					{#each columns as col}
						<Table.Cell class={col.class ?? ''}>
							{#if children}
								{@render children({ row, col })}
							{:else}
								{row[col.key] ?? '-'}
							{/if}
						</Table.Cell>
					{/each}
				</Table.Row>
				{#if expandedRow && expandedRowId === row[rowIdKey]}
					<Table.Row>
						<Table.Cell colspan={columns.length} class="p-0">
							{@render expandedRow({ row })}
						</Table.Cell>
					</Table.Row>
				{/if}
			{/each}
			{#if paged.length === 0}
				<Table.Row>
					<Table.Cell colspan={columns.length} class="text-muted-foreground py-8 text-center">
						{search ? 'No results match your search.' : 'No data.'}
					</Table.Cell>
				</Table.Row>
			{/if}
		</Table.Body>
	</Table.Root>

	<!-- Pagination footer -->
	{#if sorted.length > 0}
		<div
			class="border-border text-muted-foreground flex items-center justify-between border-t px-3 py-2 text-xs"
		>
			<span>{showingFrom}-{showingTo} of {sorted.length}</span>
			<div class="flex items-center gap-3">
				<span>Per page:</span>
				{#each [10, 25, 50] as size}
					<button
						class="rounded px-1.5 py-0.5 transition-colors {pageSize === size
							? 'bg-primary text-primary-foreground'
							: 'hover:text-foreground'}"
						onclick={() => {
							pageSize = size;
							currentPage = 0;
						}}
					>
						{size}
					</button>
				{/each}
				<span class="text-border mx-1">|</span>
				<button
					class="hover:text-foreground disabled:opacity-30"
					disabled={currentPage === 0}
					onclick={() => currentPage--}
				>
					<ChevronLeftIcon class="h-4 w-4" />
				</button>
				<span>{currentPage + 1}/{totalPages}</span>
				<button
					class="hover:text-foreground disabled:opacity-30"
					disabled={currentPage >= totalPages - 1}
					onclick={() => currentPage++}
				>
					<ChevronRightIcon class="h-4 w-4" />
				</button>
			</div>
		</div>
	{/if}
</div>
