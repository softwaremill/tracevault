<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import * as Select from '$lib/components/ui/select/index.js';
	import StatCard from '$lib/components/StatCard.svelte';
	import ActivityIcon from '@lucide/svelte/icons/activity';
	import MonitorPlayIcon from '@lucide/svelte/icons/monitor-play';
	import GitCommitHorizontalIcon from '@lucide/svelte/icons/git-commit-horizontal';
	import ZapIcon from '@lucide/svelte/icons/zap';

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

	function onRepoSelect(val: string) {
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
			<Select.Root type="single" value={repoId} onValueChange={(v) => onRepoSelect(v)}>
				<Select.Trigger size="sm">
					<span data-slot="select-value">{repoId ? repos.find(r => r.id === repoId)?.name ?? 'All repos' : 'All repos'}</span>
				</Select.Trigger>
				<Select.Content>
					<Select.Item value="">All repos</Select.Item>
					{#each repos as repo}
						<Select.Item value={repo.id}>{repo.name}</Select.Item>
					{/each}
				</Select.Content>
			</Select.Root>
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
		<div class="grid grid-cols-2 gap-4 md:grid-cols-4">
			<StatCard label="Active Sessions" value={fmtNum(stats.active_sessions)} icon={ActivityIcon} color="#10b981" />
			<StatCard label="Total Sessions" value={fmtNum(stats.total_sessions)} icon={MonitorPlayIcon} color="#3b82f6" />
			<StatCard label="Total Commits" value={fmtNum(stats.total_commits)} icon={GitCommitHorizontalIcon} color="#f59e0b" />
			<StatCard label="Total Events" value={fmtNum(stats.total_events)} icon={ZapIcon} color="#8b5cf6" />
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
