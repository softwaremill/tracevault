<script lang="ts">
	import HelpTip from '$lib/components/HelpTip.svelte';

	interface Props {
		savingsUsd: number;
		savingsPct: number;
		href: string;
	}

	let { savingsUsd, savingsPct, href }: Props = $props();
</script>

<a {href} class="bg-background hover:bg-muted/50 block rounded-lg border p-4 transition-colors">
	<div class="text-muted-foreground text-[11px] font-medium uppercase tracking-wide">
		Cache Savings<HelpTip text="Money saved by reusing cached prompt tokens instead of re-processing them at full input rate." />
	</div>
	<div class="mt-1 text-2xl font-bold text-green-500">
		${savingsUsd.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}
	</div>
	<div class="text-muted-foreground mt-1 text-xs">
		{savingsPct.toFixed(1)}% of total cost
	</div>
</a>
