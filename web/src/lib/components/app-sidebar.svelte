<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { auth } from '$lib/stores/auth';
	import * as Sidebar from '$lib/components/ui/sidebar/index.js';
	import { Button } from '$lib/components/ui/button/index.js';

	let authState: { user: { email: string; org_name: string; role: string } | null } = $state({
		user: null
	});
	auth.subscribe((s) => (authState = s));

	const navItems = [
		{ href: '/repos', label: 'Repos' },
		{ href: '/traces', label: 'Traces' },
		{ href: '/analytics', label: 'Analytics' },
		{ href: '/settings', label: 'Settings' }
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

	function isActive(href: string): boolean {
		return $page.url.pathname.startsWith(href);
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

<Sidebar.Root>
	<Sidebar.Header class="p-4">
		<a href="/repos" class="text-lg font-semibold">TraceVault</a>
		{#if authState.user}
			<p class="text-xs text-muted-foreground">{authState.user.org_name}</p>
		{/if}
	</Sidebar.Header>
	<Sidebar.Content>
		<Sidebar.Group>
			<Sidebar.GroupContent>
				<Sidebar.Menu>
					{#each navItems as item}
						<Sidebar.MenuItem>
							<Sidebar.MenuButton isActive={isActive(item.href)}>
								{#snippet child({ props })}
									<a href={item.href} {...props}>{item.label}</a>
								{/snippet}
							</Sidebar.MenuButton>
						</Sidebar.MenuItem>
					{/each}
				</Sidebar.Menu>
			</Sidebar.GroupContent>
		</Sidebar.Group>
		{#if $page.url.pathname.startsWith('/analytics')}
			<Sidebar.Group>
				<Sidebar.GroupLabel>Analytics</Sidebar.GroupLabel>
				<Sidebar.GroupContent>
					<Sidebar.Menu>
						{#each analyticsSubItems as item}
							<Sidebar.MenuItem>
								<Sidebar.MenuButton isActive={$page.url.pathname === item.href}>
									{#snippet child({ props })}
										<a href={item.href} {...props}>{item.label}</a>
									{/snippet}
								</Sidebar.MenuButton>
							</Sidebar.MenuItem>
						{/each}
					</Sidebar.Menu>
				</Sidebar.GroupContent>
			</Sidebar.Group>
		{/if}
	</Sidebar.Content>
	<Sidebar.Footer class="p-4">
		{#if authState.user}
			<p class="text-sm truncate">{authState.user.email}</p>
		{/if}
		<Button variant="outline" size="sm" class="w-full mt-2" onclick={handleLogout}>
			Log out
		</Button>
	</Sidebar.Footer>
</Sidebar.Root>
