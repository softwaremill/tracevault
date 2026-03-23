<script lang="ts">
	import TranscriptRecord from './TranscriptRecord.svelte';

	interface TranscriptRecordData {
		record_type: string;
		timestamp: string | null;
		content_types: string[];
		tool_name: string | null;
		text: string | null;
		usage: {
			input_tokens: number;
			output_tokens: number;
			cache_read_tokens: number;
			cache_write_tokens: number;
			cost_usd: number;
		} | null;
		model: string | null;
	}

	interface Props {
		records: TranscriptRecordData[];
	}

	let { records }: Props = $props();

	const TYPE_COLORS: Record<string, string> = {
		user: '#3ecf8e',
		assistant: '#a78bfa',
		progress: '#f6b144',
		system: '#f06565',
		'file-history-snapshot': '#4f6ef7',
		'last-prompt': '#6b7594'
	};

	let typeCounts = $derived(
		records.reduce(
			(acc, r) => {
				acc[r.record_type] = (acc[r.record_type] || 0) + 1;
				return acc;
			},
			{} as Record<string, number>
		)
	);

	let activeFilters = $state<Set<string>>(new Set());

	function toggleFilter(type: string) {
		const next = new Set(activeFilters);
		if (next.has(type)) {
			next.delete(type);
		} else {
			next.add(type);
		}
		activeFilters = next;
	}

	let filteredRecords = $derived(
		activeFilters.size === 0
			? records
			: records.filter((r) => activeFilters.has(r.record_type))
	);
</script>

<div>
	<div class="mb-3 flex items-center gap-2">
		<span class="text-sm font-semibold">Transcript ({records.length} records)</span>
	</div>

	<div class="mb-3 flex flex-wrap gap-2">
		{#each Object.entries(typeCounts) as [type, count]}
			{@const color = TYPE_COLORS[type] || '#6b7594'}
			{@const isActive = activeFilters.size === 0 || activeFilters.has(type)}
			<button
				class="rounded-full border px-2.5 py-1 text-[11px] transition-opacity"
				style="background: {color}15; color: {color}; border-color: {color}30; opacity: {isActive ? 1 : 0.4}"
				onclick={() => toggleFilter(type)}
			>
				{type} ({count})
			</button>
		{/each}
	</div>

	<div class="border-border overflow-hidden rounded-lg border">
		{#each filteredRecords as record}
			<TranscriptRecord {record} />
		{/each}

		{#if filteredRecords.length === 0}
			<div class="text-muted-foreground p-4 text-center text-sm">No matching records</div>
		{/if}
	</div>
</div>
