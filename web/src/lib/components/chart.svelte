<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { Chart as ChartJS, registerables, type ChartType, type ChartData, type ChartOptions } from 'chart.js';

	ChartJS.register(...registerables);

	interface Props {
		type: ChartType;
		data: ChartData;
		options?: ChartOptions;
	}

	let { type, data, options = {} }: Props = $props();

	let canvas: HTMLCanvasElement;
	let chart: ChartJS | null = null;

	onMount(() => {
		chart = new ChartJS(canvas, { type, data, options });
	});

	$effect(() => {
		if (chart) {
			chart.data = data;
			if (options) chart.options = options;
			chart.update();
		}
	});

	onDestroy(() => {
		chart?.destroy();
	});
</script>

<canvas bind:this={canvas}></canvas>
