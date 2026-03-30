<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import { auth } from '$lib/stores/auth';
	import { orgStore } from '$lib/stores/org';
	import * as Table from '$lib/components/ui/table/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Input } from '$lib/components/ui/input/index.js';
	import { Label } from '$lib/components/ui/label/index.js';
	import * as Dialog from '$lib/components/ui/dialog/index.js';
	import * as Select from '$lib/components/ui/select/index.js';
	import * as Alert from '$lib/components/ui/alert/index.js';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import { formatDate, formatDateTime } from '$lib/utils/date';

	interface Member {
		id: string;
		email: string;
		name: string | null;
		role: string;
		created_at: string;
	}

	const slug = $derived($page.params.slug);

	let orgState: { current: { role: string } | null } = $state({ current: null });
	orgStore.subscribe((s) => (orgState = s));

	let authState: { user: { user_id: string } | null } = $state({ user: null });
	auth.subscribe((s) => (authState = s));

	interface Invite {
		id: string;
		email: string;
		role: string;
		status: string;
		expires_at: string;
		created_at: string;
	}

	interface InvitationRequest {
		id: string;
		email: string;
		name: string | null;
		status: string;
		created_at: string;
	}

	let members: Member[] = $state([]);
	let invites: Invite[] = $state([]);
	let invRequests: InvitationRequest[] = $state([]);
	let loading = $state(true);
	let error = $state('');

	let inviteSearch = $state('');
	let invitePage = $state(0);
	const invitePageSize = 10;

	const filteredInvites = $derived(
		invites.filter((i) => {
			const q = inviteSearch.toLowerCase();
			return !q || i.email.toLowerCase().includes(q) || i.role.includes(q) || i.status.includes(q);
		})
	);
	const inviteTotalPages = $derived(Math.max(1, Math.ceil(filteredInvites.length / invitePageSize)));
	const pagedInvites = $derived(filteredInvites.slice(invitePage * invitePageSize, (invitePage + 1) * invitePageSize));

	let inviteOpen = $state(false);
	let inviteEmail = $state('');
	let inviteRole = $state('developer');
	let inviteError = $state('');
	let inviteLoading = $state(false);
	let inviteUrl = $state('');
	let copied = $state(false);

	const isOwner = $derived(orgState.current?.role === 'owner');
	const isAdmin = $derived(
		orgState.current?.role === 'owner' || orgState.current?.role === 'admin'
	);

	onMount(() => {
		loadMembers();
		loadInvites();
		loadInvitationRequests();
	});

	async function loadMembers() {
		try {
			members = await api.get<Member[]>(`/api/v1/orgs/${slug}/members`);
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load members';
		} finally {
			loading = false;
		}
	}

	async function handleInvite(e: Event) {
		e.preventDefault();
		inviteError = '';
		inviteUrl = '';
		inviteLoading = true;
		try {
			const res = await api.post<{ invite_url: string }>(`/api/v1/orgs/${slug}/invites`, {
				email: inviteEmail,
				role: inviteRole
			});
			inviteUrl = res.invite_url;
			inviteEmail = '';
			inviteRole = 'developer';
			await loadInvites();
		} catch (err) {
			inviteError = err instanceof Error ? err.message : 'Failed to create invite';
		} finally {
			inviteLoading = false;
		}
	}

	async function loadInvites() {
		try {
			invites = await api.get<Invite[]>(`/api/v1/orgs/${slug}/invites`);
		} catch {
			// User may not have permission
		}
	}

	async function revokeInvite(id: string) {
		try {
			await api.delete(`/api/v1/orgs/${slug}/invites/${id}`);
			await loadInvites();
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to revoke invite';
		}
	}

	async function removeMember(userId: string) {
		if (!confirm('Remove this member?')) return;
		try {
			await api.delete(`/api/v1/orgs/${slug}/members/${userId}`);
			await loadMembers();
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to remove member';
		}
	}

	async function changeRole(userId: string, newRole: string) {
		try {
			await api.put(`/api/v1/orgs/${slug}/members/${userId}/role`, {
				role: newRole
			});
			await loadMembers();
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to change role';
		}
	}

	async function loadInvitationRequests() {
		try {
			invRequests = await api.get<InvitationRequest[]>(`/api/v1/orgs/${slug}/invitation-requests`);
		} catch {
			// User may not have permission — that's fine
		}
	}

	async function approveRequest(id: string) {
		try {
			await api.post(`/api/v1/orgs/${slug}/invitation-requests/${id}/approve`);
			await loadInvitationRequests();
			await loadMembers();
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to approve request';
		}
	}

	async function rejectRequest(id: string) {
		try {
			await api.post(`/api/v1/orgs/${slug}/invitation-requests/${id}/reject`);
			await loadInvitationRequests();
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to reject request';
		}
	}

	function roleColor(role: string): { bg: string; color: string; border: string } {
		switch (role) {
			case 'owner': return { bg: 'rgba(246,177,68,0.12)', color: '#f6b144', border: 'rgba(246,177,68,0.25)' };
			case 'admin': return { bg: 'rgba(167,139,250,0.12)', color: '#a78bfa', border: 'rgba(167,139,250,0.25)' };
			default: return { bg: 'rgba(79,110,247,0.12)', color: '#4f6ef7', border: 'rgba(79,110,247,0.25)' };
		}
	}

	function statusColor(status: string): { bg: string; color: string; border: string } {
		switch (status) {
			case 'approved':
			case 'accepted': return { bg: 'rgba(62,207,142,0.12)', color: '#3ecf8e', border: 'rgba(62,207,142,0.25)' };
			case 'rejected':
			case 'revoked': return { bg: 'rgba(240,101,101,0.12)', color: '#f06565', border: 'rgba(240,101,101,0.25)' };
			case 'expired': return { bg: 'rgba(156,163,175,0.12)', color: '#9ca3af', border: 'rgba(156,163,175,0.25)' };
			default: return { bg: 'rgba(79,110,247,0.12)', color: '#4f6ef7', border: 'rgba(79,110,247,0.25)' };
		}
	}
</script>

<svelte:head>
	<title>Members - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	<div class="flex items-center gap-2">
		<a href="/orgs/{slug}/settings" class="text-muted-foreground hover:text-foreground">Organizations</a>
		<span class="text-muted-foreground">/</span>
		<h1 class="text-2xl font-bold">{slug}</h1>
	</div>

	<div class="flex gap-2 text-sm border-b pb-2">
		<a href="/orgs/{slug}/settings/org" class="text-muted-foreground hover:underline">General</a>
		<a href="/orgs/{slug}/settings/members" class="font-semibold underline">Members</a>
		<a href="/orgs/{slug}/settings/api-keys" class="text-muted-foreground hover:underline">API Keys</a>
	</div>

	{#if error}
		<ErrorState message={error} />
	{:else}
	<div class="flex items-center justify-between">
		<h2 class="text-sm font-semibold uppercase tracking-wide text-muted-foreground">Members</h2>
		{#if isAdmin}
			<Dialog.Root bind:open={inviteOpen} onOpenChange={(open) => { if (!open) { inviteUrl = ''; inviteError = ''; copied = false; } }}>
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
					{#if inviteUrl}
						<div class="grid gap-2">
							<Label>Invite link</Label>
							<div class="flex gap-2">
								<Input value={inviteUrl} readonly class="flex-1" />
								<Button variant="outline" onclick={() => { navigator.clipboard.writeText(inviteUrl); copied = true; setTimeout(() => { copied = false; }, 2000); }}>
									{copied ? 'Copied!' : 'Copy'}
								</Button>
							</div>
							<p class="text-xs text-muted-foreground">Share this link with the invited user.</p>
						</div>
						<Dialog.Footer>
							<Button onclick={() => { inviteOpen = false; inviteUrl = ''; }}>Done</Button>
						</Dialog.Footer>
					{:else}
						<form onsubmit={handleInvite} class="grid gap-4">
							<div class="grid gap-2">
								<Label for="invite_email">Email</Label>
								<Input id="invite_email" type="email" bind:value={inviteEmail} required />
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
									{inviteLoading ? 'Sending...' : 'Send invite'}
								</Button>
							</Dialog.Footer>
						</form>
					{/if}
				</Dialog.Content>
			</Dialog.Root>
		{/if}
	</div>

	{#if loading}
		<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm">
			<span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"></span>
			Loading...
		</div>
	{:else if members.length === 0}
		<p class="text-muted-foreground text-sm">No members found.</p>
	{:else}
		<Table.Root>
			<Table.Header>
				<Table.Row>
					<Table.Head class="text-xs">Email</Table.Head>
					<Table.Head class="text-xs">Name</Table.Head>
					<Table.Head class="text-xs">Role</Table.Head>
					<Table.Head class="text-xs">Joined</Table.Head>
					{#if isOwner}
						<Table.Head class="text-xs">Actions</Table.Head>
					{/if}
				</Table.Row>
			</Table.Header>
			<Table.Body>
				{#each members as member}
					<Table.Row class="hover:bg-muted/40 transition-colors">
						<Table.Cell class="text-xs">{member.email}</Table.Cell>
						<Table.Cell class="text-xs">{member.name ?? '-'}</Table.Cell>
						<Table.Cell class="text-xs">
							{@const rc = roleColor(member.role)}
							<span class="rounded-full px-2 py-0.5 text-[10px]" style="background: {rc.bg}; color: {rc.color}; border: 1px solid {rc.border}">{member.role}</span>
						</Table.Cell>
						<Table.Cell class="text-xs">{formatDate(member.created_at)}</Table.Cell>
						{#if isOwner}
							<Table.Cell class="text-xs">
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

	<!-- Invites -->
	{#if isAdmin && invites.length > 0}
		<div class="space-y-3 pt-4">
			<div class="flex items-center justify-between">
				<h2 class="text-sm font-semibold uppercase tracking-wide text-muted-foreground">Invites</h2>
				<Input
					type="text"
					placeholder="Search invites..."
					class="w-64 h-8 text-xs"
					value={inviteSearch}
					oninput={(e) => { inviteSearch = e.currentTarget.value; invitePage = 0; }}
				/>
			</div>
			<Table.Root>
				<Table.Header>
					<Table.Row>
						<Table.Head class="text-xs">Email</Table.Head>
						<Table.Head class="text-xs">Role</Table.Head>
						<Table.Head class="text-xs">Status</Table.Head>
						<Table.Head class="text-xs">Expires</Table.Head>
						<Table.Head class="text-xs">Sent</Table.Head>
						<Table.Head class="text-xs">Actions</Table.Head>
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each pagedInvites as invite}
						<Table.Row class="hover:bg-muted/40 transition-colors">
							<Table.Cell class="text-xs">{invite.email}</Table.Cell>
							<Table.Cell class="text-xs">
								{@const rc = roleColor(invite.role)}
								<span class="rounded-full px-2 py-0.5 text-[10px]" style="background: {rc.bg}; color: {rc.color}; border: 1px solid {rc.border}">{invite.role}</span>
							</Table.Cell>
							<Table.Cell class="text-xs">
								{@const sc = statusColor(invite.status)}
								<span class="rounded-full px-2 py-0.5 text-[10px]" style="background: {sc.bg}; color: {sc.color}; border: 1px solid {sc.border}">{invite.status}</span>
							</Table.Cell>
							<Table.Cell class="text-xs">{formatDateTime(invite.expires_at)}</Table.Cell>
							<Table.Cell class="text-xs">{formatDateTime(invite.created_at)}</Table.Cell>
							<Table.Cell class="text-xs">
								{#if invite.status === 'pending'}
									<Button size="sm" variant="destructive" onclick={() => revokeInvite(invite.id)}>Revoke</Button>
								{/if}
							</Table.Cell>
						</Table.Row>
					{/each}
				</Table.Body>
			</Table.Root>
			{#if inviteTotalPages > 1}
				<div class="flex items-center justify-between pt-2">
					<p class="text-xs text-muted-foreground">{filteredInvites.length} invite{filteredInvites.length === 1 ? '' : 's'}</p>
					<div class="flex items-center gap-2">
						<Button variant="outline" size="sm" disabled={invitePage === 0} onclick={() => invitePage--}>Previous</Button>
						<span class="text-xs text-muted-foreground">{invitePage + 1} / {inviteTotalPages}</span>
						<Button variant="outline" size="sm" disabled={invitePage >= inviteTotalPages - 1} onclick={() => invitePage++}>Next</Button>
					</div>
				</div>
			{/if}
		</div>
	{/if}

	<!-- Invitation Requests -->
	{#if isAdmin && invRequests.length > 0}
		<div class="space-y-3 pt-4">
			<h2 class="text-sm font-semibold uppercase tracking-wide text-muted-foreground">Invitation Requests</h2>
			<Table.Root>
				<Table.Header>
					<Table.Row>
						<Table.Head class="text-xs">Email</Table.Head>
						<Table.Head class="text-xs">Name</Table.Head>
						<Table.Head class="text-xs">Status</Table.Head>
						<Table.Head class="text-xs">Requested</Table.Head>
						<Table.Head class="text-xs">Actions</Table.Head>
					</Table.Row>
				</Table.Header>
				<Table.Body>
					{#each invRequests as req}
						<Table.Row class="hover:bg-muted/40 transition-colors">
							<Table.Cell class="text-xs">{req.email}</Table.Cell>
							<Table.Cell class="text-xs">{req.name ?? '-'}</Table.Cell>
							<Table.Cell class="text-xs">
								{@const sc = statusColor(req.status)}
								<span class="rounded-full px-2 py-0.5 text-[10px]" style="background: {sc.bg}; color: {sc.color}; border: 1px solid {sc.border}">{req.status}</span>
							</Table.Cell>
							<Table.Cell class="text-xs">{formatDate(req.created_at)}</Table.Cell>
							<Table.Cell class="text-xs">
								{#if req.status === 'pending'}
									<div class="flex gap-1">
										<Button size="sm" onclick={() => approveRequest(req.id)}>Approve</Button>
										<Button size="sm" variant="destructive" onclick={() => rejectRequest(req.id)}>Reject</Button>
									</div>
								{/if}
							</Table.Cell>
						</Table.Row>
					{/each}
				</Table.Body>
			</Table.Root>
		</div>
	{/if}
	{/if}
</div>
