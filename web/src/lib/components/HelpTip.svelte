<script lang="ts">
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';

	interface Props {
		text: string;
	}

	let { text }: Props = $props();
</script>

<Tooltip.Root>
	<Tooltip.Trigger>
		<span class="ml-1 cursor-help text-xs font-normal" style="color: #4f6ef7">?</span>
	</Tooltip.Trigger>
	<Tooltip.Portal>
		<Tooltip.Content class="max-w-xs text-xs">
			{text}
		</Tooltip.Content>
	</Tooltip.Portal>
</Tooltip.Root>
