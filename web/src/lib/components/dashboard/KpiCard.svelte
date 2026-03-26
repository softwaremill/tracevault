<script lang="ts">
	import HelpTip from '$lib/components/HelpTip.svelte';

	interface Props {
		label: string;
		value: string;
		trend: number;
		trendLabel?: string;
		sparkline: number[];
		href: string;
		color?: string;
		tooltip?: string;
	}

	let {
		label,
		value,
		trend,
		trendLabel,
		sparkline,
		href,
		color = '#3b82f6',
		tooltip
	}: Props = $props();

	const trendPositive = $derived(trend >= 0);
	const trendDisplay = $derived(
		trendLabel ?? (trend >= 0 ? `+${trend.toFixed(1)}%` : `${trend.toFixed(1)}%`)
	);

	function sparklinePath(data: number[]): string {
		if (data.length < 2) return '';
		const max = Math.max(...data);
		const min = Math.min(...data);
		const range = max - min || 1;
		const w = 120;
		const h = 28;
		const step = w / (data.length - 1);
		return data
			.map((v, i) => {
				const x = i * step;
				const y = h - ((v - min) / range) * h;
				return `${i === 0 ? 'M' : 'L'}${x.toFixed(1)},${y.toFixed(1)}`;
			})
			.join(' ');
	}
</script>

<a
	{href}
	class="bg-background hover:bg-muted/50 block rounded-lg border p-4 transition-colors"
>
	<div class="text-muted-foreground text-[11px] font-medium uppercase tracking-wide">
		{label}
		{#if tooltip}<HelpTip text={tooltip} />{/if}
	</div>
	<div class="mt-1 text-2xl font-semibold">{value}</div>
	<div class="mt-0.5 text-xs font-medium {trendPositive ? 'text-green-500' : 'text-amber-500'}">
		{trendDisplay}
	</div>
	{#if sparkline.length >= 2}
		<svg viewBox="0 0 120 28" class="mt-2 h-7 w-full" preserveAspectRatio="none">
			<path d={sparklinePath(sparkline)} fill="none" stroke={color} stroke-width="1.5" />
		</svg>
	{/if}
</a>
