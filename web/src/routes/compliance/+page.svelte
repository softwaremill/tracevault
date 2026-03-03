<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/api';
	import { auth } from '$lib/stores/auth';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Table from '$lib/components/ui/table/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Button } from '$lib/components/ui/button/index.js';

	interface ComplianceSettings {
		org_id: string;
		retention_days: number;
		signing_enabled: boolean;
		compliance_mode: string;
		chain_verification_interval_hours: number | null;
		created_at: string;
		updated_at: string;
	}

	interface ChainStatus {
		status: string;
		total_commits: number;
		verified_commits: number;
		errors: unknown[] | null;
		last_verified_at: string | null;
	}

	interface AuditLogEntry {
		id: string;
		actor_id: string | null;
		action: string;
		resource_type: string;
		resource_id: string | null;
		details: Record<string, unknown> | null;
		created_at: string;
	}

	interface AuditLogResponse {
		entries: AuditLogEntry[];
		total: number;
		page: number;
		per_page: number;
	}

	interface MemberResponse {
		id: string;
		email: string;
		name: string | null;
		role: string;
	}

	let orgId = $state('');
	let settings: ComplianceSettings | null = $state(null);
	let chainStatus: ChainStatus | null = $state(null);
	let recentAudit: AuditLogEntry[] = $state([]);
	let members: MemberResponse[] = $state([]);
	let loading = $state(true);
	let verifying = $state(false);
	let error = $state('');

	let authState: { user: { org_id: string; role: string } | null } = $state({ user: null });
	auth.subscribe((s) => (authState = s));

	onMount(async () => {
		if (!authState.user) return;
		orgId = authState.user.org_id;
		await loadAll();
	});

	async function loadAll() {
		loading = true;
		error = '';
		try {
			const [s, cs, al, m] = await Promise.all([
				api.get<ComplianceSettings>(`/api/v1/orgs/${orgId}/compliance`),
				api.get<ChainStatus>(`/api/v1/orgs/${orgId}/compliance/chain-status`),
				api.get<AuditLogResponse>(`/api/v1/orgs/${orgId}/audit-log?per_page=20`),
				api.get<MemberResponse[]>(`/api/v1/orgs/${orgId}/members`).catch(() => [])
			]);
			settings = s;
			chainStatus = cs;
			recentAudit = al.entries;
			members = m;
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load compliance data';
		} finally {
			loading = false;
		}
	}

	async function runVerification() {
		verifying = true;
		try {
			chainStatus = await api.post<ChainStatus>(
				`/api/v1/orgs/${orgId}/compliance/verify-chain`
			);
		} catch (err) {
			error = err instanceof Error ? err.message : 'Verification failed';
		} finally {
			verifying = false;
		}
	}

	function formatDate(iso: string): string {
		return new Date(iso).toLocaleString();
	}

	function roleCounts(): Record<string, number> {
		const counts: Record<string, number> = {};
		for (const m of members) {
			counts[m.role] = (counts[m.role] || 0) + 1;
		}
		return counts;
	}

	function modeLabel(mode: string): string {
		const labels: Record<string, string> = {
			none: 'None',
			sox: 'SOX',
			pci_dss: 'PCI-DSS',
			sr_11_7: 'SR 11-7',
			custom: 'Custom'
		};
		return labels[mode] || mode;
	}
</script>

<svelte:head>
	<title>Compliance Dashboard - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	<h1 class="text-2xl font-bold">Compliance Dashboard</h1>

	{#if loading}
		<p class="text-muted-foreground">Loading...</p>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else}
		<!-- Top Cards Row -->
		<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
			<!-- Chain Integrity -->
			<Card.Root>
				<Card.Header class="pb-2">
					<Card.Title class="text-sm font-medium">Chain Integrity</Card.Title>
				</Card.Header>
				<Card.Content>
					{#if chainStatus}
						<div class="flex items-center gap-2 mb-2">
							<Badge
								variant={chainStatus.status === 'pass'
									? 'success'
									: chainStatus.status === 'never_run'
										? 'secondary'
										: 'error'}
							>
								{chainStatus.status === 'pass'
									? 'Verified'
									: chainStatus.status === 'never_run'
										? 'Not Verified'
										: 'Failed'}
							</Badge>
						</div>
						{#if chainStatus.status !== 'never_run'}
							<p class="text-xs text-muted-foreground">
								{chainStatus.verified_commits}/{chainStatus.total_commits} commits verified
							</p>
						{/if}
						{#if chainStatus.last_verified_at}
							<p class="text-xs text-muted-foreground">
								Last: {formatDate(chainStatus.last_verified_at)}
							</p>
						{/if}
						<Button
							size="sm"
							variant="outline"
							class="mt-2"
							onclick={runVerification}
							disabled={verifying}
						>
							{verifying ? 'Verifying...' : 'Verify Now'}
						</Button>
					{/if}
				</Card.Content>
			</Card.Root>

			<!-- Compliance Mode -->
			<Card.Root>
				<Card.Header class="pb-2">
					<Card.Title class="text-sm font-medium">Compliance Mode</Card.Title>
				</Card.Header>
				<Card.Content>
					{#if settings}
						<Badge
							variant={settings.compliance_mode !== 'none' ? 'success' : 'secondary'}
							class="text-lg px-3 py-1"
						>
							{modeLabel(settings.compliance_mode)}
						</Badge>
						<p class="text-xs text-muted-foreground mt-2">
							Signing: {settings.signing_enabled ? 'Enabled' : 'Disabled'}
						</p>
						<a href="/compliance/settings" class="text-xs text-primary hover:underline"
							>Configure</a
						>
					{/if}
				</Card.Content>
			</Card.Root>

			<!-- Retention -->
			<Card.Root>
				<Card.Header class="pb-2">
					<Card.Title class="text-sm font-medium">Data Retention</Card.Title>
				</Card.Header>
				<Card.Content>
					{#if settings}
						<p class="text-2xl font-bold">{settings.retention_days} days</p>
						<p class="text-xs text-muted-foreground">
							{Math.floor(settings.retention_days / 365)} year{settings.retention_days >=
							730
								? 's'
								: ''} retention policy
						</p>
					{/if}
				</Card.Content>
			</Card.Root>
		</div>

		<!-- Role Distribution -->
		<Card.Root>
			<Card.Header>
				<Card.Title>Role Distribution</Card.Title>
			</Card.Header>
			<Card.Content>
				<div class="flex flex-wrap gap-4">
					{#each Object.entries(roleCounts()) as [role, count]}
						<div class="flex items-center gap-2">
							<Badge variant="outline">{role}</Badge>
							<span class="text-sm font-medium">{count}</span>
						</div>
					{/each}
					{#if members.length === 0}
						<p class="text-sm text-muted-foreground">No members data available</p>
					{/if}
				</div>
			</Card.Content>
		</Card.Root>

		<!-- Recent Audit Log -->
		<Card.Root>
			<Card.Header class="flex flex-row items-center justify-between">
				<Card.Title>Recent Audit Log</Card.Title>
				<a href="/compliance/audit-log">
					<Button variant="outline" size="sm">View All</Button>
				</a>
			</Card.Header>
			<Card.Content>
				{#if recentAudit.length === 0}
					<p class="text-muted-foreground">No audit log entries yet.</p>
				{:else}
					<Table.Root>
						<Table.Header>
							<Table.Row>
								<Table.Head>Time</Table.Head>
								<Table.Head>Action</Table.Head>
								<Table.Head>Resource</Table.Head>
								<Table.Head>Details</Table.Head>
							</Table.Row>
						</Table.Header>
						<Table.Body>
							{#each recentAudit as entry}
								<Table.Row>
									<Table.Cell class="text-xs"
										>{formatDate(entry.created_at)}</Table.Cell
									>
									<Table.Cell>
										<Badge variant="action">{entry.action}</Badge>
									</Table.Cell>
									<Table.Cell class="text-xs font-mono"
										>{entry.resource_type}</Table.Cell
									>
									<Table.Cell class="text-xs max-w-xs truncate">
										{entry.details ? JSON.stringify(entry.details) : '-'}
									</Table.Cell>
								</Table.Row>
							{/each}
						</Table.Body>
					</Table.Root>
				{/if}
			</Card.Content>
		</Card.Root>
	{/if}
</div>
