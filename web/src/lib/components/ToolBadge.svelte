<script lang="ts">
	let { tool }: { tool: string } = $props();

	const displayNames: Record<string, string> = {
		'claude-code': 'Claude Code',
		cursor: 'Cursor',
		codex: 'Codex'
	};
</script>

<span
	class="inline-flex items-center gap-1 rounded-full bg-muted px-2 py-0.5 text-xs font-medium"
>
	{displayNames[tool] || tool}
</span>
