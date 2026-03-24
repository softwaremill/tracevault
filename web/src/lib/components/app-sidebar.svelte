<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { browser } from '$app/environment';
	import { auth } from '$lib/stores/auth';
	import { orgStore } from '$lib/stores/org';
	import { features } from '$lib/stores/features';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';
	import {
		LayoutDashboard,
		FolderGit2,
		GitCommitHorizontal,
		BarChart3,
		ShieldCheck,
		Settings,
		LogOut,
		ChevronsLeft,
		ChevronsRight
	} from '@lucide/svelte';

	let authState: { user: { email: string } | null } = $state({
		user: null
	});
	auth.subscribe((s) => (authState = s));

	let edition = $state('community');
	features.subscribe((f) => (edition = f.edition));

	interface OrgInfo {
		org_name: string;
		display_name: string | null;
	}
	let orgCurrent = $state<OrgInfo | null>(null);
	let orgAll = $state<OrgInfo[]>([]);
	$effect(() => {
		const unsub = orgStore.subscribe((s) => {
			orgCurrent = s.current;
			orgAll = s.all;
		});
		return unsub;
	});

	const slug = $derived(orgCurrent?.org_name ?? $page.params.slug ?? '');
	let showOrgMenu = $state(false);

	let expanded = $state(false);

	if (browser) {
		expanded = localStorage.getItem('sidebar-expanded') === 'true';
	}

	function toggleExpanded() {
		expanded = !expanded;
		if (browser) {
			localStorage.setItem('sidebar-expanded', String(expanded));
		}
	}

	const navItems = $derived([
		{ href: `/orgs/${slug}/dashboard`, label: 'Dashboard', icon: LayoutDashboard },
		{ href: `/orgs/${slug}/repos`, label: 'Repos', icon: FolderGit2 },
		{ href: `/orgs/${slug}/traces`, label: 'Traces', icon: GitCommitHorizontal },
		{ href: `/orgs/${slug}/analytics`, label: 'Analytics', icon: BarChart3 },
		{ href: `/orgs/${slug}/compliance`, label: 'Compliance', icon: ShieldCheck },
		{ href: `/orgs/${slug}/settings`, label: 'Settings', icon: Settings }
	]);

	const analyticsSubItems = $derived([
		{ href: `/orgs/${slug}/analytics`, label: 'Overview' },
		{ href: `/orgs/${slug}/analytics/tokens`, label: 'Tokens' },
		{ href: `/orgs/${slug}/analytics/models`, label: 'Models' },
		{ href: `/orgs/${slug}/analytics/authors`, label: 'Authors' },
		{ href: `/orgs/${slug}/analytics/attribution`, label: 'Attribution' },
		{ href: `/orgs/${slug}/analytics/sessions`, label: 'Sessions' },
		{ href: `/orgs/${slug}/analytics/cost`, label: 'Cost' }
	]);

	const complianceSubItems = $derived([
		{ href: `/orgs/${slug}/compliance`, label: 'Dashboard' },
		{ href: `/orgs/${slug}/compliance/audit-log`, label: 'Audit Log' },
		{ href: `/orgs/${slug}/compliance/settings`, label: 'Settings' }
	]);

	const settingsSubItems = $derived([
		{ href: `/orgs/${slug}/settings`, label: 'Organizations' },
		{ href: `/orgs/${slug}/settings/pricing`, label: 'Pricing' },
		{ href: `/orgs/${slug}/settings/llm`, label: 'LLM' }
	]);

	function isActive(href: string): boolean {
		return $page.url.pathname === href || $page.url.pathname.startsWith(href + '/');
	}

	function isExactActive(href: string): boolean {
		return $page.url.pathname === href;
	}

	async function handleLogout() {
		try {
			const { api } = await import('$lib/api');
			await api.post('/api/v1/auth/logout');
		} catch {
			// ignore
		}
		orgStore.clear();
		auth.logout();
		goto('/auth/login');
	}
</script>

<aside
	class="flex flex-col border-r bg-sidebar text-sidebar-foreground transition-all duration-200 ease-in-out {expanded ? 'w-60' : 'w-14'}"
	style="min-height: 100vh;"
>
	<!-- Header: Logo -->
	<div class="flex h-14 items-center border-b px-3 {expanded ? 'justify-between' : 'justify-center'}">
		<a href="/orgs/{slug}/repos" class="flex items-center gap-2">
			<img src="/logo.png" alt="TraceVault" class="h-8 w-8 rounded-lg" />
			{#if expanded}
				<div class="flex flex-col">
					<span class="text-lg font-semibold leading-tight">TraceVault</span>
					<span class="text-[10px] uppercase tracking-wider {edition === 'enterprise' ? 'text-primary' : 'text-muted-foreground'}">{edition}</span>
				</div>
			{/if}
		</a>
		{#if expanded}
			<button onclick={toggleExpanded} class="text-muted-foreground hover:text-foreground p-1">
				<ChevronsLeft class="h-4 w-4" />
			</button>
		{/if}
	</div>

	<!-- Org Switcher -->
	{#if orgAll.length > 0}
		<div class="border-b px-2 py-2">
			{#if expanded}
				<div class="relative">
					<button
						onclick={() => (showOrgMenu = !showOrgMenu)}
						class="flex w-full items-center justify-between rounded-md px-3 py-2 text-sm font-medium hover:bg-sidebar-accent"
					>
						<span class="flex items-center gap-2 truncate">
							<img src="https://github.com/{slug}.png?size=40" alt="" class="h-5 w-5 rounded" />
							{orgCurrent?.display_name || orgCurrent?.org_name || slug}
						</span>
						<svg class="h-4 w-4 shrink-0 text-muted-foreground" viewBox="0 0 16 16" fill="currentColor">
							<path d="M4.427 7.427l3.396 3.396a.25.25 0 00.354 0l3.396-3.396A.25.25 0 0011.396 7H4.604a.25.25 0 00-.177.427z" />
						</svg>
					</button>
					{#if showOrgMenu}
						<div class="absolute left-0 right-0 top-full z-50 mt-1 rounded-md border bg-popover p-1 shadow-md">
							{#each orgAll as org}
								<a
									href="/orgs/{org.org_name}/repos"
									onclick={() => (showOrgMenu = false)}
									class="flex items-center gap-2 rounded-sm px-2 py-1.5 text-sm hover:bg-accent
										{org.org_name === slug ? 'font-semibold' : ''}"
								>
									<img src="https://github.com/{org.org_name}.png?size=40" alt="" class="h-5 w-5 rounded" />
									{org.display_name || org.org_name}
								</a>
							{/each}
							{#if edition === 'enterprise'}
								<div class="border-t mt-1 pt-1">
									<a
										href="/orgs?create=1"
										onclick={() => (showOrgMenu = false)}
										class="flex items-center gap-2 rounded-sm px-2 py-1.5 text-sm text-muted-foreground hover:bg-accent hover:text-foreground"
									>
										<svg class="h-4 w-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/></svg>
										Create organization
									</a>
								</div>
							{/if}
						</div>
					{/if}
				</div>
			{:else}
				<Tooltip.Root>
					<Tooltip.Trigger>
						<div class="flex h-10 w-10 items-center justify-center">
							<img src="https://github.com/{slug}.png?size=40" alt="" class="h-8 w-8 rounded" />
						</div>
					</Tooltip.Trigger>
					<Tooltip.Content side="right">
						{orgCurrent?.display_name || orgCurrent?.org_name || slug}
					</Tooltip.Content>
				</Tooltip.Root>
			{/if}
		</div>
	{/if}

	<!-- Navigation -->
	<nav class="flex-1 py-3 px-2 space-y-1">
		{#each navItems as item}
			{@const active = isActive(item.href)}
			{#if expanded}
				<a
					href={item.href}
					class="flex items-center gap-3 rounded-md px-3 py-2 text-sm font-medium transition-colors
						{active ? 'bg-primary text-primary-foreground' : 'text-sidebar-foreground hover:bg-sidebar-accent'}"
				>
					<item.icon class="h-5 w-5 shrink-0" />
					<span>{item.label}</span>
				</a>

				{#if item.href === `/orgs/${slug}/analytics` && active}
					<div class="ml-8 space-y-0.5 mt-0.5">
						{#each analyticsSubItems as sub}
							<a
								href={sub.href}
								class="block rounded-md px-3 py-1.5 text-xs font-medium transition-colors
									{isExactActive(sub.href) ? 'text-primary font-semibold' : 'text-muted-foreground hover:text-foreground'}"
							>
								{sub.label}
							</a>
						{/each}
					</div>
				{/if}

				{#if item.href === `/orgs/${slug}/compliance` && active}
					<div class="ml-8 space-y-0.5 mt-0.5">
						{#each complianceSubItems as sub}
							<a
								href={sub.href}
								class="block rounded-md px-3 py-1.5 text-xs font-medium transition-colors
									{isExactActive(sub.href) ? 'text-primary font-semibold' : 'text-muted-foreground hover:text-foreground'}"
							>
								{sub.label}
							</a>
						{/each}
					</div>
				{/if}

				{#if item.href === `/orgs/${slug}/settings` && active}
					<div class="ml-8 space-y-0.5 mt-0.5">
						{#each settingsSubItems as sub}
							<a
								href={sub.href}
								class="block rounded-md px-3 py-1.5 text-xs font-medium transition-colors
									{isExactActive(sub.href) ? 'text-primary font-semibold' : 'text-muted-foreground hover:text-foreground'}"
							>
								{sub.label}
							</a>
						{/each}
					</div>
				{/if}
			{:else}
				<Tooltip.Root>
					<Tooltip.Trigger>
						<a
							href={item.href}
							class="flex h-10 w-10 items-center justify-center rounded-md transition-colors
								{active ? 'bg-primary text-primary-foreground' : 'text-sidebar-foreground hover:bg-sidebar-accent'}"
						>
							<item.icon class="h-5 w-5" />
						</a>
					</Tooltip.Trigger>
					<Tooltip.Content side="right">
						{item.label}
					</Tooltip.Content>
				</Tooltip.Root>
			{/if}
		{/each}
	</nav>

	<!-- Footer -->
	<div class="border-t p-2">
		{#if !expanded}
			<button onclick={toggleExpanded} class="flex h-10 w-10 items-center justify-center rounded-md text-muted-foreground hover:text-foreground hover:bg-sidebar-accent mx-auto">
				<ChevronsRight class="h-4 w-4" />
			</button>
		{:else}
			{#if authState.user}
				<p class="text-xs text-muted-foreground truncate px-3 py-1">{authState.user.email}</p>
			{/if}
			<Button variant="ghost" size="sm" class="w-full justify-start gap-2" onclick={handleLogout}>
				<LogOut class="h-4 w-4" />
				Log out
			</Button>
		{/if}
	</div>
</aside>
