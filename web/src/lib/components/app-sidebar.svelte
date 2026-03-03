<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { browser } from '$app/environment';
	import { auth } from '$lib/stores/auth';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Tooltip from '$lib/components/ui/tooltip/index.js';
	import {
		FolderGit2,
		GitCommitHorizontal,
		BarChart3,
		ShieldCheck,
		Settings,
		LogOut,
		ChevronsLeft,
		ChevronsRight
	} from '@lucide/svelte';

	let authState: { user: { email: string; org_name: string; role: string } | null } = $state({
		user: null
	});
	auth.subscribe((s) => (authState = s));

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

	const navItems = [
		{ href: '/repos', label: 'Repos', icon: FolderGit2 },
		{ href: '/traces', label: 'Traces', icon: GitCommitHorizontal },
		{ href: '/analytics', label: 'Analytics', icon: BarChart3 },
		{ href: '/compliance', label: 'Compliance', icon: ShieldCheck },
		{ href: '/settings', label: 'Settings', icon: Settings }
	];

	const analyticsSubItems = [
		{ href: '/analytics', label: 'Overview' },
		{ href: '/analytics/tokens', label: 'Tokens' },
		{ href: '/analytics/models', label: 'Models' },
		{ href: '/analytics/authors', label: 'Authors' },
		{ href: '/analytics/attribution', label: 'Attribution' },
		{ href: '/analytics/sessions', label: 'Sessions' },
		{ href: '/analytics/cost', label: 'Cost' }
	];

	const complianceSubItems = [
		{ href: '/compliance', label: 'Dashboard' },
		{ href: '/compliance/audit-log', label: 'Audit Log' },
		{ href: '/compliance/settings', label: 'Settings' }
	];

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
		<a href="/repos" class="flex items-center gap-2">
			<img src="/logo.png" alt="TraceVault" class="h-8 w-8 rounded-lg" />
			{#if expanded}
				<span class="text-lg font-semibold">TraceVault</span>
			{/if}
		</a>
		{#if expanded}
			<button onclick={toggleExpanded} class="text-muted-foreground hover:text-foreground p-1">
				<ChevronsLeft class="h-4 w-4" />
			</button>
		{/if}
	</div>

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

				{#if item.href === '/analytics' && active}
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

				{#if item.href === '/compliance' && active}
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
