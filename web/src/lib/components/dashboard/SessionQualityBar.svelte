<script lang="ts">
	import HelpTip from '$lib/components/HelpTip.svelte';

	interface Props {
		avgDurationMs: number;
		avgToolCalls: number;
		avgCompactions: number;
	}

	let { avgDurationMs, avgToolCalls, avgCompactions }: Props = $props();

	function formatDuration(ms: number): string {
		const totalSec = Math.round(ms / 1000);
		const m = Math.floor(totalSec / 60);
		const s = totalSec % 60;
		return m > 0 ? `${m}m ${s}s` : `${s}s`;
	}
</script>

<div class="bg-background flex items-center gap-8 rounded-lg border px-4 py-3">
	<div>
		<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Avg Duration<HelpTip text="Average wall-clock duration of sessions in this period." /></div>
		<div class="mt-0.5 text-sm font-semibold">{formatDuration(avgDurationMs)}</div>
	</div>
	<div>
		<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Tool Calls / Session<HelpTip text="Average number of tool invocations (file edits, reads, bash commands, etc.) per session." /></div>
		<div class="mt-0.5 text-sm font-semibold">{avgToolCalls.toFixed(1)}</div>
	</div>
	<div>
		<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Compactions / Session<HelpTip text="Average number of context window compactions per session. High values indicate long sessions that exceeded the context limit." /></div>
		<div class="mt-0.5 text-sm font-semibold">{avgCompactions.toFixed(1)}</div>
	</div>
</div>
