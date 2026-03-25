<script lang="ts">
	import type { Component } from 'svelte';

	interface Props {
		label: string;
		value: string;
		icon: Component;
		color?: string;
		secondary?: string;
	}

	let { label, value, icon: Icon, color = '#3b82f6', secondary }: Props = $props();
</script>

<div class="bg-background rounded-lg border border-border p-4">
	<div class="flex items-center gap-3">
		<div
			class="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg"
			style="background: color-mix(in oklch, {color} 15%, transparent)"
		>
			<Icon class="h-4 w-4" style="color: {color}" />
		</div>
		<div class="min-w-0">
			<div class="text-muted-foreground text-[10px] font-semibold uppercase tracking-wider">
				{label}
			</div>
			<div class="text-xl font-bold leading-tight">{value}</div>
			{#if secondary}
				<div class="text-muted-foreground mt-0.5 text-[11px]">{secondary}</div>
			{/if}
		</div>
	</div>
</div>
