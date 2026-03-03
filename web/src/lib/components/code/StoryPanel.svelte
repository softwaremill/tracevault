<script lang="ts">
	import type { StoryResponse } from '$lib/types/code';
	import { marked } from 'marked';

	let {
		story,
		loading,
		onClose,
		onRegenerate
	}: {
		story: StoryResponse | null;
		loading: boolean;
		onClose: () => void;
		onRegenerate: () => void;
	} = $props();

	const renderedMarkdown = $derived.by(() => {
		if (!story) return '';
		return marked.parse(story.story) as string;
	});
</script>

{#if story || loading}
	<div class="fixed right-0 top-0 z-50 flex h-full w-[480px] flex-col border-l bg-card shadow-xl">
		<div class="flex items-center justify-between border-b p-4">
			<div>
				<h3 class="text-lg font-semibold">
					{#if story}
						{story.function_name}
					{:else}
						Generating story...
					{/if}
				</h3>
				{#if story}
					<p class="text-sm text-muted-foreground">
						{story.kind} -- Lines {story.line_range[0]}-{story.line_range[1]}
						{#if story.cached}-- Cached{/if}
					</p>
				{/if}
			</div>
			<div class="flex gap-2">
				{#if story}
					<button
						onclick={onRegenerate}
						class="rounded border px-2 py-1 text-sm hover:bg-accent"
					>
						Regenerate
					</button>
				{/if}
				<button onclick={onClose} class="rounded border px-2 py-1 text-sm hover:bg-accent">
					X
				</button>
			</div>
		</div>

		<div class="flex-1 overflow-y-auto p-4">
			{#if loading}
				<div class="flex items-center gap-2 text-muted-foreground">
					<div
						class="h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
					></div>
					Analyzing code history and generating story...
				</div>
			{:else if story}
				<div class="prose prose-sm max-w-none dark:prose-invert">
					{@html renderedMarkdown}
				</div>
				<div class="mt-6 space-y-1 border-t pt-4 text-xs text-muted-foreground">
					<p>Commits analyzed: {story.commits_analyzed.length}</p>
					<p>Generated: {new Date(story.generated_at).toLocaleString()}</p>
				</div>
			{/if}
		</div>
	</div>
{/if}
