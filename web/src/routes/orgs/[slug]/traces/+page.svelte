<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/stores';

	const slug = $derived($page.params.slug);

	$effect(() => {
		goto(`/orgs/${slug}/traces/sessions`, { replaceState: true });
	});
</script>
