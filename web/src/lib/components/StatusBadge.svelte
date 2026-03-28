<script lang="ts">
	type SessionDisplayStatus = 'active' | 'completed' | 'stale';

	let { status }: { status: string } = $props();

	const styles: Record<SessionDisplayStatus, { bg: string; text: string; label: string }> = {
		active: { bg: 'bg-green-500/10', text: 'text-green-600', label: 'Active' },
		completed: { bg: 'bg-blue-500/10', text: 'text-blue-600', label: 'Completed' },
		stale: { bg: 'bg-yellow-500/10', text: 'text-yellow-600', label: 'Stale' }
	};

	const style = $derived(styles[status as SessionDisplayStatus] || styles.active);
</script>

<span
	class="inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium {style.bg} {style.text}"
>
	{style.label}
</span>
