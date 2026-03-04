<script lang="ts">
	import hljs from 'highlight.js/lib/core';
	import rust from 'highlight.js/lib/languages/rust';
	import typescript from 'highlight.js/lib/languages/typescript';
	import javascript from 'highlight.js/lib/languages/javascript';
	import python from 'highlight.js/lib/languages/python';
	import go from 'highlight.js/lib/languages/go';
	import java from 'highlight.js/lib/languages/java';
	import scala from 'highlight.js/lib/languages/scala';
	import json from 'highlight.js/lib/languages/json';
	import yaml from 'highlight.js/lib/languages/yaml';
	import sql from 'highlight.js/lib/languages/sql';
	import bash from 'highlight.js/lib/languages/bash';
	import css from 'highlight.js/lib/languages/css';
	import xml from 'highlight.js/lib/languages/xml';
	import markdown from 'highlight.js/lib/languages/markdown';
	import 'highlight.js/styles/github-dark.css';

	hljs.registerLanguage('rust', rust);
	hljs.registerLanguage('typescript', typescript);
	hljs.registerLanguage('javascript', javascript);
	hljs.registerLanguage('python', python);
	hljs.registerLanguage('go', go);
	hljs.registerLanguage('java', java);
	hljs.registerLanguage('scala', scala);
	hljs.registerLanguage('json', json);
	hljs.registerLanguage('yaml', yaml);
	hljs.registerLanguage('sql', sql);
	hljs.registerLanguage('bash', bash);
	hljs.registerLanguage('css', css);
	hljs.registerLanguage('xml', xml);
	hljs.registerLanguage('markdown', markdown);

	let {
		content,
		language,
		onLineClick
	}: {
		content: string;
		language: string | null;
		onLineClick?: (line: number) => void;
	} = $props();

	let selectedLine = $state<number | null>(null);

	const highlighted = $derived.by(() => {
		if (language && hljs.getLanguage(language)) {
			return hljs.highlight(content, { language }).value;
		}
		return hljs.highlightAuto(content).value;
	});

	const lines = $derived(highlighted.split('\n'));

	function handleLineClick(lineNum: number) {
		if (!onLineClick) return;
		selectedLine = lineNum;
		onLineClick(lineNum);
	}
</script>

<div class="code-view hljs overflow-x-auto rounded-lg font-mono text-sm">
	<table class="w-full border-collapse">
		<tbody>
			{#each lines as line, i}
				<tr class="hover:bg-white/5 {selectedLine === i + 1 ? 'bg-yellow-500/20' : ''}">
					<td
						class="w-12 cursor-pointer select-none border-r border-gray-700 py-0 pl-4 pr-4 text-right text-gray-500"
						onclick={() => handleLineClick(i + 1)}
						role="button"
						tabindex="0"
						onkeydown={(e) => {
							if (e.key === 'Enter' || e.key === ' ') handleLineClick(i + 1);
						}}
					>
						{i + 1}
					</td>
					<td class="whitespace-pre py-0 pl-4">
						{@html line || ' '}
					</td>
				</tr>
			{/each}
		</tbody>
	</table>
</div>

<style>
	.code-view {
		/* Ensure highlight.js theme colors aren't overridden by Tailwind base styles */
		color: #e6edf3;
		background: #0d1117;
	}
	.code-view :global(.hljs-keyword),
	.code-view :global(.hljs-selector-tag),
	.code-view :global(.hljs-literal),
	.code-view :global(.hljs-section),
	.code-view :global(.hljs-link) {
		color: #ff7b72;
	}
	.code-view :global(.hljs-string),
	.code-view :global(.hljs-addition) {
		color: #a5d6ff;
	}
	.code-view :global(.hljs-title),
	.code-view :global(.hljs-title.class_),
	.code-view :global(.hljs-title.function_) {
		color: #d2a8ff;
	}
	.code-view :global(.hljs-type),
	.code-view :global(.hljs-built_in),
	.code-view :global(.hljs-builtin-name),
	.code-view :global(.hljs-selector-id),
	.code-view :global(.hljs-selector-attr),
	.code-view :global(.hljs-selector-pseudo),
	.code-view :global(.hljs-params) {
		color: #ffa657;
	}
	.code-view :global(.hljs-number),
	.code-view :global(.hljs-symbol) {
		color: #79c0ff;
	}
	.code-view :global(.hljs-comment),
	.code-view :global(.hljs-quote),
	.code-view :global(.hljs-deletion) {
		color: #8b949e;
	}
	.code-view :global(.hljs-meta),
	.code-view :global(.hljs-attr) {
		color: #79c0ff;
	}
	.code-view :global(.hljs-variable),
	.code-view :global(.hljs-template-variable) {
		color: #ffa657;
	}
	.code-view :global(.hljs-punctuation) {
		color: #e6edf3;
	}
</style>
