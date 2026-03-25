<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { api } from '$lib/api';
	import * as Select from '$lib/components/ui/select/index.js';

	const TOOL_COLORS: Record<string, string> = {
		Edit: '#f59e0b',
		Bash: '#06b6d4',
		Read: '#a855f6',
		Grep: '#22c55e',
		Agent: '#3b82f6'
	};

	interface TimelineItem {
		type: 'event' | 'commit';
		event_id?: string;
		session_id?: string;
		session_short_id?: string;
		event_type?: string;
		tool_name?: string;
		file_path?: string;
		commit_sha?: string;
		branch?: string;
		author?: string;
		timestamp: string;
	}

	let items: TimelineItem[] = $state([]);
	let loading = $state(true);
	let error = $state('');
	let toolFilter = $state('');
	let sessionFilter = $state('');

	const slug = $derived($page.params.slug);

	async function fetchData(search: string) {
		loading = true;
		error = '';
		try {
			items = await api.get<TimelineItem[]>(
				`/api/v1/orgs/${slug}/traces/timeline` + (search ? '?' + search : '')
			);
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load timeline';
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		const search = $page.url.search.replace(/^\?/, '');
		fetchData(search);
	});

	function applyFilters() {
		const params = new URLSearchParams($page.url.search);
		if (toolFilter) {
			params.set('tool_name', toolFilter);
		} else {
			params.delete('tool_name');
		}
		if (sessionFilter) {
			params.set('session_id', sessionFilter);
		} else {
			params.delete('session_id');
		}
		const qs = params.toString();
		goto(`/orgs/${slug}/traces/timeline` + (qs ? '?' + qs : ''), { replaceState: true });
	}

	function toolColor(name: string | undefined): string {
		if (!name) return '#94a3b8';
		return TOOL_COLORS[name] ?? '#94a3b8';
	}

	function fmtTime(iso: string): string {
		return new Date(iso).toLocaleString();
	}

	const uniqueSessions = $derived(
		[...new Set(items.filter((i) => i.session_short_id).map((i) => i.session_short_id!))]
	);

	const uniqueTools = $derived(
		[...new Set(items.filter((i) => i.tool_name).map((i) => i.tool_name!))]
	);
</script>

<svelte:head>
	<title>Timeline - TraceVault</title>
</svelte:head>

<div class="space-y-4">
	<h1 class="text-2xl font-bold">Timeline</h1>

	<!-- Filter row -->
	<div class="flex flex-wrap items-center gap-3">
		<Select.Root type="single" value={toolFilter} onValueChange={(v) => { toolFilter = v; applyFilters(); }}>
			<Select.Trigger size="sm">
				<span data-slot="select-value">{toolFilter || 'All tools'}</span>
			</Select.Trigger>
			<Select.Content>
				<Select.Item value="">All tools</Select.Item>
				{#each uniqueTools as tool}
					<Select.Item value={tool}>{tool}</Select.Item>
				{/each}
			</Select.Content>
		</Select.Root>
		<Select.Root type="single" value={sessionFilter} onValueChange={(v) => { sessionFilter = v; applyFilters(); }}>
			<Select.Trigger size="sm">
				<span data-slot="select-value">{sessionFilter || 'All sessions'}</span>
			</Select.Trigger>
			<Select.Content>
				<Select.Item value="">All sessions</Select.Item>
				{#each uniqueSessions as sid}
					<Select.Item value={sid}>{sid}</Select.Item>
				{/each}
			</Select.Content>
		</Select.Root>
	</div>

	{#if loading}
		<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm">
			<span
				class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"
			></span>
			Loading...
		</div>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if items.length === 0}
		<p class="text-muted-foreground">No timeline events.</p>
	{:else}
		<div class="divide-border divide-y">
			{#each items as item}
				{#if item.type === 'commit'}
					<!-- Commit row -->
					<div class="flex items-center gap-3 bg-muted/30 px-3 py-2">
						<svg
							class="h-4 w-4 shrink-0 text-orange-400"
							viewBox="0 0 16 16"
							fill="currentColor"
						>
							<path
								d="M11.93 8.5a4.002 4.002 0 0 1-7.86 0H.75a.75.75 0 0 1 0-1.5h3.32a4.002 4.002 0 0 1 7.86 0h3.32a.75.75 0 0 1 0 1.5Zm-1.43-.25a2.5 2.5 0 1 0-5 0 2.5 2.5 0 0 0 5 0Z"
							/>
						</svg>
						<a
							href="/orgs/{slug}/traces/commits/{item.commit_sha}"
							class="font-mono text-xs underline"
						>
							{item.commit_sha?.slice(0, 8)}
						</a>
						{#if item.branch}
							<span
								class="rounded-full px-2 py-0.5 text-[10px]"
								style="background: rgba(79,110,247,0.12); color: #4f6ef7; border: 1px solid rgba(79,110,247,0.25)"
								>{item.branch}</span
							>
						{/if}
						<span class="text-muted-foreground text-xs">{item.author}</span>
						<span class="text-muted-foreground ml-auto text-[10px]">{fmtTime(item.timestamp)}</span
						>
					</div>
				{:else}
					<!-- Event row -->
					<div class="flex items-center gap-3 px-3 py-2">
						{#if item.session_short_id}
							<a
								href="/orgs/{slug}/traces/sessions/{item.session_id}"
								class="rounded-full px-2 py-0.5 text-[10px] font-mono"
								style="background: rgba(129,140,248,0.12); color: #818cf8; border: 1px solid rgba(129,140,248,0.25)"
							>
								{item.session_short_id}
							</a>
						{/if}
						<span class="flex items-center gap-1.5 text-xs">
							<span
								class="inline-block h-2 w-2 rounded-full"
								style="background: {toolColor(item.tool_name)}"
							></span>
							<span class="w-12 font-medium" style="color: {toolColor(item.tool_name)}">
								{item.tool_name ?? '-'}
							</span>
						</span>
						<span class="text-muted-foreground truncate font-mono text-xs">
							{item.file_path ?? ''}
						</span>
						<span class="text-muted-foreground ml-auto shrink-0 text-[10px]"
							>{fmtTime(item.timestamp)}</span
						>
					</div>
				{/if}
			{/each}
		</div>
	{/if}
</div>
