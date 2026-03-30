<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { browser } from '$app/environment';
	import { auth } from '$lib/stores/auth';
	import { orgStore } from '$lib/stores/org';
	import { features } from '$lib/stores/features';
	import { ChevronsLeft } from '@lucide/svelte';
	import SidebarNav from './sidebar/SidebarNav.svelte';
	import SidebarOrgSwitcher from './sidebar/SidebarOrgSwitcher.svelte';
	import SidebarFooter from './sidebar/SidebarFooter.svelte';

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
	class="flex flex-col border-r bg-sidebar text-sidebar-foreground transition-all duration-200 ease-in-out {expanded
		? 'w-60'
		: 'w-14'}"
	style="min-height: 100vh;"
>
	<!-- Header: Logo -->
	<div
		class="flex h-14 items-center border-b px-3 {expanded ? 'justify-between' : 'justify-center'}"
	>
		<a href="/orgs/{slug}/dashboard" class="flex items-center gap-2">
			<img src="/logo.png" alt="TraceVault" class="h-8 w-8 rounded-lg" />
			{#if expanded}
				<div class="flex flex-col">
					<span class="text-lg font-semibold leading-tight">TraceVault</span>
					<span
						class="text-[10px] uppercase tracking-wider {edition === 'enterprise'
							? 'text-primary'
							: 'text-muted-foreground'}">{edition}</span
					>
				</div>
			{/if}
		</a>
		{#if expanded}
			<button onclick={toggleExpanded} class="text-muted-foreground hover:text-foreground p-1">
				<ChevronsLeft class="h-4 w-4" />
			</button>
		{/if}
	</div>

	<SidebarOrgSwitcher {expanded} {slug} {orgAll} {orgCurrent} {edition} />
	<SidebarNav {expanded} {slug} />
	<SidebarFooter
		{expanded}
		userEmail={authState.user?.email ?? null}
		onLogout={handleLogout}
		onExpand={toggleExpanded}
	/>
</aside>
