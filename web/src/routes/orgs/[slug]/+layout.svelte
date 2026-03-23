<script lang="ts">
	import { page } from '$app/stores';
	import { orgStore } from '$lib/stores/org';
	import { goto } from '$app/navigation';
	import AppLayout from '$lib/components/app-layout.svelte';

	interface OrgItem {
		org_id: string;
		org_name: string;
		display_name: string | null;
		role: string;
	}

	let { children } = $props();
	let loaded = $state(false);
	let lastSlug = $state('');

	$effect(() => {
		const slug = $page.params.slug;
		if (slug && slug !== lastSlug) {
			lastSlug = slug;
			loadOrg(slug);
		}
	});

	async function loadOrg(slug: string) {
		loaded = false;
		const orgs = await orgStore.loadOrgs();
		const current = orgs.find((o: OrgItem) => o.org_name === slug);
		if (!current) {
			goto('/orgs');
			return;
		}
		orgStore.setCurrent(current);
		loaded = true;
	}
</script>

{#if loaded}
	<AppLayout>
		{@render children()}
	</AppLayout>
{/if}
