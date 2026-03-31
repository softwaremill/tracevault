<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import { features } from '$lib/stores/features';
	import { formatDateTime } from '$lib/utils/date';
	import EnterpriseUpgrade from '$lib/components/enterprise-upgrade.svelte';
	import DataTable from '$lib/components/DataTable.svelte';
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
		total_sessions: number;
		verified_sessions: number;
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

	const slug = $derived($page.params.slug);

	let settings: ComplianceSettings | null = $state(null);
	let chainStatus: ChainStatus | null = $state(null);
	let recentAudit: AuditLogEntry[] = $state([]);
	let members: MemberResponse[] = $state([]);
	let loading = $state(true);
	let verifying = $state(false);
	let error = $state('');

	onMount(async () => {
		await loadAll();
	});

	async function loadAll() {
		loading = true;
		error = '';
		try {
			const [s, cs, al, m] = await Promise.all([
				api.get<ComplianceSettings>(`/api/v1/orgs/${slug}/compliance`),
				api.get<ChainStatus>(`/api/v1/orgs/${slug}/compliance/chain-status`),
				api.get<AuditLogResponse>(`/api/v1/orgs/${slug}/audit-log?per_page=20`),
				api.get<MemberResponse[]>(`/api/v1/orgs/${slug}/members`).catch(() => [])
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
				`/api/v1/orgs/${slug}/compliance/verify-chain`
			);
		} catch (err) {
			error = err instanceof Error ? err.message : 'Verification failed';
		} finally {
			verifying = false;
		}
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

{#if !$features.loaded}
	<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm"><span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"></span>Loading...</div>
{:else if $features.compliance}
<div class="space-y-6">
	<h1 class="text-2xl font-bold">Compliance Dashboard</h1>

	{#if loading}
		<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm"><span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"></span>Loading...</div>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else}
		<!-- Top Cards Row -->
		<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
			<!-- Chain Integrity -->
			<div class="border-border overflow-hidden rounded-lg border">
				<div class="bg-muted/30 px-4 py-3 text-sm font-semibold">Chain Integrity</div>
				<div class="p-4 space-y-3">
					{#if chainStatus}
						<div class="flex items-center gap-2 mb-2">
							{#if chainStatus.status === 'pass'}
								<span class="rounded-full px-2 py-0.5 text-[10px]" style="background: rgba(62,207,142,0.12); color: #3ecf8e; border: 1px solid rgba(62,207,142,0.25)">Verified</span>
							{:else if chainStatus.status === 'never_run'}
								<span class="rounded-full px-2 py-0.5 text-[10px]" style="background: rgba(79,110,247,0.12); color: #4f6ef7; border: 1px solid rgba(79,110,247,0.25)">Not Verified</span>
							{:else}
								<span class="rounded-full px-2 py-0.5 text-[10px]" style="background: rgba(240,101,101,0.12); color: #f06565; border: 1px solid rgba(240,101,101,0.25)">Failed</span>
							{/if}
						</div>
						{#if chainStatus.status !== 'never_run'}
							<p class="text-xs text-muted-foreground">
								{chainStatus.verified_commits}/{chainStatus.total_commits} commits verified
							</p>
							<p class="text-xs text-muted-foreground">
								{chainStatus.verified_sessions}/{chainStatus.total_sessions} session snapshots verified
							</p>
						{/if}
						{#if chainStatus.last_verified_at}
							<p class="text-xs text-muted-foreground">
								Last: {formatDateTime(chainStatus.last_verified_at)}
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
				</div>
			</div>

			<!-- Compliance Mode -->
			<div class="border-border overflow-hidden rounded-lg border">
				<div class="bg-muted/30 px-4 py-3 text-sm font-semibold">Compliance Mode</div>
				<div class="p-4 space-y-3">
					{#if settings}
						{#if settings.compliance_mode !== 'none'}
							<span class="rounded-full px-2 py-0.5 text-[10px]" style="background: rgba(62,207,142,0.12); color: #3ecf8e; border: 1px solid rgba(62,207,142,0.25)">{modeLabel(settings.compliance_mode)}</span>
						{:else}
							<span class="rounded-full px-2 py-0.5 text-[10px]" style="background: rgba(79,110,247,0.12); color: #4f6ef7; border: 1px solid rgba(79,110,247,0.25)">{modeLabel(settings.compliance_mode)}</span>
						{/if}
						<p class="text-xs text-muted-foreground mt-2">
							Signing: {settings.signing_enabled ? 'Enabled' : 'Disabled'}
						</p>
						<a href="/orgs/{slug}/compliance/settings" class="text-xs text-primary hover:underline"
							>Configure</a
						>
					{/if}
				</div>
			</div>

			<!-- Retention -->
			<div class="border-border overflow-hidden rounded-lg border">
				<div class="bg-muted/30 px-4 py-3 text-sm font-semibold">Data Retention</div>
				<div class="p-4 space-y-3">
					{#if settings}
						<p class="text-2xl font-bold">{settings.retention_days} days</p>
						<p class="text-xs text-muted-foreground">
							{Math.floor(settings.retention_days / 365)} year{settings.retention_days >=
							730
								? 's'
								: ''} retention policy
						</p>
					{/if}
				</div>
			</div>
		</div>

		<!-- Role Distribution -->
		<div class="border-border overflow-hidden rounded-lg border">
			<div class="bg-muted/30 px-4 py-3 text-sm font-semibold">Role Distribution</div>
			<div class="p-4 space-y-3">
				<div class="flex flex-wrap gap-4">
					{#each Object.entries(roleCounts()) as [role, count]}
						<div class="flex items-center gap-2">
							<span class="rounded-full px-2 py-0.5 text-[10px]" style="background: rgba(167,139,250,0.12); color: #a78bfa; border: 1px solid rgba(167,139,250,0.25)">{role}</span>
							<span class="text-sm font-medium">{count}</span>
						</div>
					{/each}
					{#if members.length === 0}
						<p class="text-sm text-muted-foreground">No members data available</p>
					{/if}
				</div>
			</div>
		</div>

		<!-- Recent Audit Log -->
		<div>
			<div class="flex items-center justify-between px-4 py-3">
				<span class="text-sm font-semibold">Recent Audit Log</span>
				<a href="/orgs/{slug}/compliance/audit-log">
					<Button variant="outline" size="sm">View All</Button>
				</a>
			</div>
			<DataTable
				columns={[
					{ key: 'created_at', label: 'Time', sortable: true },
					{ key: 'action', label: 'Action', sortable: true },
					{ key: 'resource_type', label: 'Resource', sortable: true },
					{ key: '_details', label: 'Details' }
				]}
				rows={recentAudit.map((entry) => ({
					...entry,
					_details: entry.details ? JSON.stringify(entry.details) : '-'
				}))}
				searchKeys={['action', 'resource_type']}
				defaultSort="created_at"
				defaultSortDir="desc"
				rowIdKey="id"
			>
				{#snippet children({ row, col })}
					{#if col.key === 'created_at'}
						{formatDateTime(String(row.created_at))}
					{:else if col.key === 'action'}
						<span class="rounded-full px-2 py-0.5 text-[10px]" style="background: rgba(79,110,247,0.12); color: #4f6ef7; border: 1px solid rgba(79,110,247,0.25)">{row.action}</span>
					{:else if col.key === 'resource_type'}
						<span class="font-mono">{row.resource_type}</span>
					{:else if col.key === '_details'}
						<span class="max-w-xs truncate">{row._details}</span>
					{/if}
				{/snippet}
			</DataTable>
		</div>
	{/if}
</div>
{:else}
	<EnterpriseUpgrade feature="compliance" />
{/if}
