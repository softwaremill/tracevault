<script lang="ts">
	type Period = '7d' | '30d' | 'month';

	interface Props {
		value: Period;
		onchange: (period: Period) => void;
	}

	let { value, onchange }: Props = $props();

	const options: { label: string; value: Period }[] = [
		{ label: '7d', value: '7d' },
		{ label: '30d', value: '30d' },
		{ label: 'Month', value: 'month' }
	];
</script>

<div class="bg-muted inline-flex rounded-lg p-1">
	{#each options as opt}
		<button
			class="rounded-md px-3 py-1.5 text-sm font-medium transition-colors
				{value === opt.value
				? 'bg-background text-foreground shadow-sm'
				: 'text-muted-foreground hover:text-foreground'}"
			onclick={() => onchange(opt.value)}
		>
			{opt.label}
		</button>
	{/each}
</div>
