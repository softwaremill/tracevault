<script lang="ts">
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
		<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Avg Duration</div>
		<div class="mt-0.5 text-sm font-semibold">{formatDuration(avgDurationMs)}</div>
	</div>
	<div>
		<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Tool Calls / Session</div>
		<div class="mt-0.5 text-sm font-semibold">{avgToolCalls.toFixed(1)}</div>
	</div>
	<div>
		<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Compactions / Session</div>
		<div class="mt-0.5 text-sm font-semibold">{avgCompactions.toFixed(1)}</div>
	</div>
</div>
