<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { auth } from '$lib/stores/auth';
	import * as Table from '$lib/components/ui/table/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import * as Select from '$lib/components/ui/select/index.js';
	import * as Alert from '$lib/components/ui/alert/index.js';

	interface Member {
		id: string;
		email: string;
		name: string | null;
		role: string;
		created_at: string;
	}

	let authState: { user: { org_id: string; role: string; user_id: string } | null } = $state({
		user: null
	});
	auth.subscribe((s) => (authState = s));

	let members: Member[] = $state([]);
	let loading = $state(true);
	let error = $state('');

	let inviteOpen = $state(false);
	let inviteEmail = $state('');
	let invitePassword = $state('');
	let inviteName = $state('');
	let inviteRole = $state('developer');
	let inviteError = $state('');
	let inviteLoading = $state(false);

	const isOwner = $derived(authState.user?.role === 'owner');
	const isAdmin = $derived(
		authState.user?.role === 'owner' || authState.user?.role === 'admin'
	);

	$effect(() => {
		if (authState.user) loadMembers();
	});

	async function loadMembers() {
		if (!authState.user) return;
		try {
			members = await api.get<Member[]>(`/api/v1/orgs/${authState.user.org_id}/members`);
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load members';
		} finally {
			loading = false;
		}
	}

	async function handleInvite(e: Event) {
		e.preventDefault();
		if (!authState.user) return;
		inviteError = '';
		inviteLoading = true;
		try {
			await api.post(`/api/v1/orgs/${authState.user.org_id}/members`, {
				email: inviteEmail,
				password: invitePassword,
				name: inviteName || undefined,
				role: inviteRole
			});
			inviteOpen = false;
			inviteEmail = '';
			invitePassword = '';
			inviteName = '';
			inviteRole = 'developer';
			await loadMembers();
		} catch (err) {
			inviteError = err instanceof Error ? err.message : 'Failed to invite member';
		} finally {
			inviteLoading = false;
		}
	}

	async function removeMember(userId: string) {
		if (!authState.user) return;
		if (!confirm('Remove this member?')) return;
		try {
			await api.delete(`/api/v1/orgs/${authState.user.org_id}/members/${userId}`);
			await loadMembers();
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to remove member';
		}
	}

	async function changeRole(userId: string, newRole: string) {
		if (!authState.user) return;
		try {
			await api.put(`/api/v1/orgs/${authState.user.org_id}/members/${userId}/role`, {
				role: newRole
			});
			await loadMembers();
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to change role';
		}
	}

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleDateString();
	}
</script>

<svelte:head>
	<title>Members - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	<h1 class="text-2xl font-bold">Settings</h1>

	<div class="flex gap-2 text-sm">
		<a href="/settings" class="text-muted-foreground hover:underline">Organization</a>
		<a href="/settings/members" class="font-semibold underline">Members</a>
		<a href="/settings/api-keys" class="text-muted-foreground hover:underline">API Keys</a>
	</div>

	{#if error}
		<Alert.Root variant="destructive">
			<Alert.Title>Error</Alert.Title>
			<Alert.Description>{error}</Alert.Description>
		</Alert.Root>
	{/if}

	<div class="flex items-center justify-between">
		<h2 class="text-lg font-semibold">Members</h2>
		{#if isAdmin}
			<Dialog.Root bind:open={inviteOpen}>
				<Dialog.Trigger>
					{#snippet child({ props })}
						<Button {...props}>Invite member</Button>
					{/snippet}
				</Dialog.Trigger>
				<Dialog.Content class="sm:max-w-md">
					<Dialog.Header>
						<Dialog.Title>Invite member</Dialog.Title>
						<Dialog.Description>Add a new member to your organization.</Dialog.Description>
					</Dialog.Header>
					{#if inviteError}
						<Alert.Root variant="destructive" class="mb-2">
							<Alert.Description>{inviteError}</Alert.Description>
						</Alert.Root>
					{/if}
					<form onsubmit={handleInvite} class="grid gap-4">
						<div class="grid gap-2">
							<Label for="invite_email">Email</Label>
							<Input id="invite_email" type="email" bind:value={inviteEmail} required />
						</div>
						<div class="grid gap-2">
							<Label for="invite_password">Password</Label>
							<Input id="invite_password" type="password" bind:value={invitePassword} required minlength={8} />
						</div>
						<div class="grid gap-2">
							<Label for="invite_name">Name</Label>
							<Input id="invite_name" bind:value={inviteName} />
						</div>
						<div class="grid gap-2">
							<Label>Role</Label>
							<Select.Root type="single" value={inviteRole} onValueChange={(v) => { if (v) inviteRole = v; }}>
								<Select.Trigger>
									{inviteRole}
								</Select.Trigger>
								<Select.Content>
									<Select.Item value="developer">Developer</Select.Item>
									<Select.Item value="admin">Admin</Select.Item>
									<Select.Item value="policy_admin">Policy Admin</Select.Item>
									<Select.Item value="auditor">Auditor</Select.Item>
								</Select.Content>
							</Select.Root>
						</div>
						<Dialog.Footer>
							<Button type="submit" disabled={inviteLoading}>
								{inviteLoading ? 'Inviting...' : 'Invite'}
							</Button>
						</Dialog.Footer>
					</form>
				</Dialog.Content>
			</Dialog.Root>
		{/if}
	</div>

	{#if loading}
		<p class="text-muted-foreground">Loading...</p>
	{:else if members.length === 0}
		<p class="text-muted-foreground">No members found.</p>
	{:else}
		<Table.Root>
			<Table.Header>
				<Table.Row>
					<Table.Head>Email</Table.Head>
					<Table.Head>Name</Table.Head>
					<Table.Head>Role</Table.Head>
					<Table.Head>Joined</Table.Head>
					{#if isOwner}
						<Table.Head>Actions</Table.Head>
					{/if}
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each members as member}
					<Table.Row>
						<Table.Cell>{member.email}</Table.Cell>
						<Table.Cell>{member.name ?? '-'}</Table.Cell>
						<Table.Cell>
							<Badge variant={member.role === 'owner' ? 'default' : 'outline'}>
								{member.role}
							</Badge>
						</Table.Cell>
						<Table.Cell>{formatDate(member.created_at)}</Table.Cell>
						{#if isOwner}
							<Table.Cell>
								{#if member.role !== 'owner' && member.id !== authState.user?.user_id}
									<div class="flex gap-1">
										<Select.Root type="single" value={member.role} onValueChange={(v) => { if (v) changeRole(member.id, v); }}>
											<Select.Trigger class="w-32 h-8 text-xs">
												{member.role}
											</Select.Trigger>
											<Select.Content>
												<Select.Item value="developer">Developer</Select.Item>
												<Select.Item value="admin">Admin</Select.Item>
												<Select.Item value="policy_admin">Policy Admin</Select.Item>
												<Select.Item value="auditor">Auditor</Select.Item>
											</Select.Content>
										</Select.Root>
										<Button variant="destructive" size="sm" onclick={() => removeMember(member.id)}>
											Remove
										</Button>
									</div>
								{/if}
							</Table.Cell>
						{/if}
					</Table.Row>
				{/each}
			</Table.Body>
		</Table.Root>
	{/if}
</div>
