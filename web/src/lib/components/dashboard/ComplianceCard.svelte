<script lang="ts">
	interface Props {
		score: number;
		trend: number;
		unsignedSessions: number;
		chainVerified: boolean | null;
		href: string;
	}

	let { score, trend, unsignedSessions, chainVerified, href }: Props = $props();

	const scoreColor = $derived(
		score >= 95 ? 'text-green-500' : score >= 80 ? 'text-amber-500' : 'text-red-500'
	);
	const trendDisplay = $derived(
		trend >= 0 ? `+${trend.toFixed(1)}%` : `${trend.toFixed(1)}%`
	);
</script>

<a {href} class="bg-background hover:bg-muted/50 block rounded-lg border p-4 transition-colors">
	<div class="text-muted-foreground text-[11px] font-medium uppercase tracking-wide">
		Compliance
	</div>
	<div class="mt-1 text-3xl font-bold {scoreColor}">{score.toFixed(0)}%</div>
	<div class="mt-0.5 text-xs font-medium {trend >= 0 ? 'text-green-500' : 'text-amber-500'}">
		{trendDisplay} from previous period
	</div>
	<div class="text-muted-foreground mt-3 space-y-1 text-xs">
		<div>{unsignedSessions} unsigned session{unsignedSessions !== 1 ? 's' : ''}</div>
		<div>
			Chain:
			{#if chainVerified === null}
				Never verified
			{:else if chainVerified}
				verified ✓
			{:else}
				failed ✗
			{/if}
		</div>
	</div>
</a>
