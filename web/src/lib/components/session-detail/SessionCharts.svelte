<script lang="ts">
	import Chart from '$lib/components/chart.svelte';
	import HelpTip from '$lib/components/HelpTip.svelte';

	interface PerCallUsage {
		index: number;
		input_tokens: number;
		output_tokens: number;
		cache_read_tokens: number;
		cache_write_tokens: number;
		cost_usd: number;
		cumulative_cost_usd: number;
		model: string;
	}

	interface TokenDistribution {
		input_tokens: number;
		output_tokens: number;
		cache_read_tokens: number;
		cache_write_tokens: number;
	}

	interface Props {
		perCall: PerCallUsage[];
		tokenDistribution: TokenDistribution;
	}

	let { perCall, tokenDistribution }: Props = $props();

	const TOKEN_COLORS = {
		cacheRead: { bg: 'rgba(62,207,142,0.5)', border: 'rgba(62,207,142,0.8)' },
		cacheWrite: { bg: 'rgba(246,177,68,0.5)', border: 'rgba(246,177,68,0.8)' },
		output: { bg: 'rgba(167,139,250,0.6)', border: 'rgba(167,139,250,0.8)' },
		input: { bg: 'rgba(79,110,247,0.5)', border: 'rgba(79,110,247,0.8)' }
	};

	function stackedBarData() {
		return {
			labels: perCall.map((c) => `#${c.index}`),
			datasets: [
				{
					label: 'Cache Read',
					data: perCall.map((c) => c.cache_read_tokens),
					backgroundColor: TOKEN_COLORS.cacheRead.bg,
					borderColor: TOKEN_COLORS.cacheRead.border,
					borderWidth: 1
				},
				{
					label: 'Cache Write',
					data: perCall.map((c) => c.cache_write_tokens),
					backgroundColor: TOKEN_COLORS.cacheWrite.bg,
					borderColor: TOKEN_COLORS.cacheWrite.border,
					borderWidth: 1
				},
				{
					label: 'Output',
					data: perCall.map((c) => c.output_tokens),
					backgroundColor: TOKEN_COLORS.output.bg,
					borderColor: TOKEN_COLORS.output.border,
					borderWidth: 1
				},
				{
					label: 'Input',
					data: perCall.map((c) => c.input_tokens),
					backgroundColor: TOKEN_COLORS.input.bg,
					borderColor: TOKEN_COLORS.input.border,
					borderWidth: 1
				}
			]
		};
	}

	function cumulativeCostData() {
		return {
			labels: perCall.map((c) => `#${c.index}`),
			datasets: [
				{
					label: 'Cumulative Cost',
					data: perCall.map((c) => c.cumulative_cost_usd),
					borderColor: '#3ecf8e',
					backgroundColor: 'rgba(62,207,142,0.08)',
					fill: true,
					tension: 0.3,
					pointRadius: 2
				}
			]
		};
	}

	function doughnutData() {
		return {
			labels: ['Cache Read', 'Cache Write', 'Output', 'Input'],
			datasets: [
				{
					data: [
						tokenDistribution.cache_read_tokens,
						tokenDistribution.cache_write_tokens,
						tokenDistribution.output_tokens,
						tokenDistribution.input_tokens
					],
					backgroundColor: [
						TOKEN_COLORS.cacheRead.bg,
						TOKEN_COLORS.cacheWrite.bg,
						TOKEN_COLORS.output.bg,
						TOKEN_COLORS.input.bg
					],
					borderColor: [
						TOKEN_COLORS.cacheRead.border,
						TOKEN_COLORS.cacheWrite.border,
						TOKEN_COLORS.output.border,
						TOKEN_COLORS.input.border
					],
					borderWidth: 1
				}
			]
		};
	}

	const stackedOptions = {
		responsive: true,
		maintainAspectRatio: false,
		scales: {
			x: { stacked: true },
			y: { stacked: true }
		},
		plugins: {
			legend: { position: 'bottom' as const, labels: { boxWidth: 12, font: { size: 11 } } }
		}
	};

	const costOptions = {
		responsive: true,
		maintainAspectRatio: false,
		scales: {
			y: {
				ticks: {
					callback: function (value: string | number) {
						const n = Number(value);
						if (n !== Math.round(n)) return '';
						return `$${n}`;
					}
				}
			}
		},
		plugins: {
			legend: { display: false }
		}
	};

	const doughnutOptions = {
		responsive: true,
		maintainAspectRatio: false,
		cutout: '65%',
		plugins: {
			legend: { position: 'bottom' as const, labels: { boxWidth: 12, font: { size: 11 } } }
		}
	};
</script>

<div class="grid grid-cols-1 gap-4 md:grid-cols-3">
	<div class="border-border rounded-lg border p-3">
		<h4 class="mb-2 text-sm font-semibold">Tokens per API Call<HelpTip text="Token breakdown for each API call in the session. Shows how token usage varies across the conversation." /></h4>
		<div style="height: 200px">
			<Chart type="bar" data={stackedBarData()} options={stackedOptions} />
		</div>
	</div>

	<div class="border-border rounded-lg border p-3">
		<h4 class="mb-2 text-sm font-semibold">Cumulative Cost<HelpTip text="Running total cost across all API calls. Steep sections indicate expensive calls." /></h4>
		<div style="height: 200px">
			<Chart type="line" data={cumulativeCostData()} options={costOptions} />
		</div>
	</div>

	<div class="border-border rounded-lg border p-3">
		<h4 class="mb-2 text-sm font-semibold">Token Distribution<HelpTip text="Overall proportion of token types. Large cache read slice indicates good cache efficiency." /></h4>
		<div style="height: 200px">
			<Chart type="doughnut" data={doughnutData()} options={doughnutOptions} />
		</div>
	</div>
</div>
