<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { api } from '$lib/api';

	let { children } = $props();

	interface Repo {
		id: string;
		name: string;
	}

	interface TracesStats {
		active_sessions: number;
		total_sessions: number;
		total_commits: number;
		total_events: number;
	}

	let repos: Repo[] = $state([]);
	let stats: TracesStats | null = $state(null);
	let statsLoading = $state(true);
	let statsError = $state('');

	const slug = $derived($page.params.slug);
	const repoId = $derived($page.url.searchParams.get('repo_id') ?? '');

	const tabs = $derived([
		{ href: `/orgs/${slug}/traces/sessions`, label: 'Sessions' },
		{ href: `/orgs/${slug}/traces/commits`, label: 'Commits' },
		{ href: `/orgs/${slug}/traces/timeline`, label: 'Timeline' },
		{ href: `/orgs/${slug}/traces/attribution`, label: 'Attribution' },
		{ href: `/orgs/${slug}/traces/branches`, label: 'Branches' }
	]);

	function isTabActive(href: string): boolean {
		return $page.url.pathname === href || $page.url.pathname.startsWith(href + '/');
	}

	function tabHref(href: string): string {
		if (repoId) return `${href}?repo_id=${repoId}`;
		return href;
	}

	async function fetchRepos() {
		try {
			repos = await api.get<Repo[]>(`/api/v1/orgs/${slug}/repos`);
		} catch {
			// ignore — repos dropdown is optional
		}
	}

	async function fetchStats() {
		statsLoading = true;
		statsError = '';
		try {
			const qs = repoId ? `?repo_id=${repoId}` : '';
			stats = await api.get<TracesStats>(`/api/v1/orgs/${slug}/traces/stats${qs}`);
		} catch (err) {
			statsError = err instanceof Error ? err.message : 'Failed to load stats';
		} finally {
			statsLoading = false;
		}
	}

	onMount(() => {
		fetchRepos();
		fetchStats();
	});

	function fmtNum(n: number): string {
		if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
		if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
		return String(n);
	}

	function onRepoChange(e: Event) {
		const select = e.target as HTMLSelectElement;
		const val = select.value;
		const url = new URL($page.url);
		if (val) {
			url.searchParams.set('repo_id', val);
		} else {
			url.searchParams.delete('repo_id');
		}
		goto(url.pathname + url.search);
	}
</script>

<svelte:head>
	<title>Traces - TraceVault</title>
</svelte:head>

<div class="space-y-4">
	<!-- Header with repo filter -->
	<div class="flex items-center justify-between">
		<h1 class="text-xl font-semibold">Traces</h1>
		{#if repos.length > 0}
			<select
				class="rounded-md border border-border bg-background px-3 py-1.5 text-sm"
				value={repoId}
				onchange={onRepoChange}
			>
				<option value="">All repos</option>
				{#each repos as repo}
					<option value={repo.id}>{repo.name}</option>
				{/each}
			</select>
		{/if}
	</div>

	<!-- Stats bar -->
	{#if statsLoading}
		<div class="text-muted-foreground flex items-center gap-2 py-4 text-sm">
			<span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"></span>
			Loading stats...
		</div>
	{:else if statsError}
		<p class="text-destructive text-sm">{statsError}</p>
	{:else if stats}
		<div class="border-border overflow-hidden rounded-lg border">
			<div class="grid grid-cols-2 gap-px md:grid-cols-4">
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Active Sessions</div>
					<div class="mt-1 text-lg font-semibold">{fmtNum(stats.active_sessions)}</div>
				</div>
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Total Sessions</div>
					<div class="mt-1 text-lg font-semibold">{fmtNum(stats.total_sessions)}</div>
				</div>
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Total Commits</div>
					<div class="mt-1 text-lg font-semibold">{fmtNum(stats.total_commits)}</div>
				</div>
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Total Events</div>
					<div class="mt-1 text-lg font-semibold">{fmtNum(stats.total_events)}</div>
				</div>
			</div>
		</div>
	{/if}

	<!-- Tab navigation -->
	<div class="border-b border-border">
		<nav class="-mb-px flex gap-6">
			{#each tabs as tab}
				<a
					href={tabHref(tab.href)}
					class="border-b-2 px-1 pb-2 text-sm font-medium transition-colors
						{isTabActive(tab.href)
							? 'border-primary text-foreground'
							: 'border-transparent text-muted-foreground hover:border-muted-foreground/40 hover:text-foreground'}"
				>
					{tab.label}
				</a>
			{/each}
		</nav>
	</div>

	<!-- Child route content -->
	{@render children()}
</div>
