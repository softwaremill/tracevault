<script lang="ts">
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import PeriodSwitcher from '$lib/components/dashboard/PeriodSwitcher.svelte';
	import KpiCard from '$lib/components/dashboard/KpiCard.svelte';
	import SessionQualityBar from '$lib/components/dashboard/SessionQualityBar.svelte';
	import ComplianceCard from '$lib/components/dashboard/ComplianceCard.svelte';
	import CacheSavingsCard from '$lib/components/dashboard/CacheSavingsCard.svelte';

	interface DashboardData {
		total_cost_usd: number;
		cost_trend_pct: number;
		cost_sparkline: number[];
		active_authors: number;
		authors_change: number;
		authors_sparkline: number[];
		total_sessions: number;
		sessions_trend_pct: number;
		sessions_sparkline: number[];
		total_tokens: number;
		tokens_trend_pct: number;
		tokens_sparkline: number[];
		avg_session_duration_ms: number;
		avg_tool_calls_per_session: number;
		avg_compactions_per_session: number;
		compliance_score_pct: number;
		compliance_trend_pct: number;
		unsigned_sessions: number;
		chain_verified: boolean | null;
		cache_savings_usd: number;
		cache_savings_pct: number;
	}

	const slug = $derived($page.params.slug);

	let period = $state<'7d' | '30d' | 'month'>('7d');
	let data = $state<DashboardData | null>(null);
	let loading = $state(true);
	let error = $state('');

	async function fetchData(p: string) {
		loading = true;
		error = '';
		try {
			data = await api.get<DashboardData>(
				`/api/v1/orgs/${slug}/dashboard?period=${p}`
			);
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load dashboard';
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		fetchData(period);
	});

	function fmtCost(v: number): string {
		return '$' + v.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 });
	}

	function fmtNum(v: number): string {
		if (v >= 1_000_000) return (v / 1_000_000).toFixed(1) + 'M';
		if (v >= 1_000) return (v / 1_000).toFixed(1) + 'k';
		return v.toLocaleString();
	}
</script>

<div class="space-y-4">
	<div class="flex items-center justify-between">
		<h1 class="text-xl font-semibold">Dashboard</h1>
		<PeriodSwitcher value={period} onchange={(p) => (period = p)} />
	</div>

	{#if loading && !data}
		<div class="text-muted-foreground py-12 text-center text-sm">Loading dashboard...</div>
	{:else if error}
		<div class="py-12 text-center text-sm text-red-500">{error}</div>
	{:else if data}
		<div class="flex gap-4">
			<!-- Left: KPI grid + session quality -->
			<div class="flex-[2] space-y-4">
				<div class="grid grid-cols-2 gap-4">
					<KpiCard
						label="Total Spend"
						value={fmtCost(data.total_cost_usd)}
						trend={data.cost_trend_pct}
						sparkline={data.cost_sparkline}
						href="/orgs/{slug}/analytics/cost"
						color="#3b82f6"
						tooltip="Total estimated cost across all sessions in the selected period, based on model pricing rates."
					/>
					<KpiCard
						label="Active Devs"
						value={data.active_authors.toString()}
						trend={data.authors_change}
						trendLabel={data.authors_change >= 0
							? `+${data.authors_change}`
							: `${data.authors_change}`}
						sparkline={data.authors_sparkline}
						href="/orgs/{slug}/analytics/authors"
						color="#8b5cf6"
						tooltip="Number of unique developers who had AI coding sessions in the selected period."
					/>
					<KpiCard
						label="Sessions"
						value={fmtNum(data.total_sessions)}
						trend={data.sessions_trend_pct}
						sparkline={data.sessions_sparkline}
						href="/orgs/{slug}/analytics/sessions"
						color="#22c55e"
						tooltip="Total AI coding sessions started in the selected period."
					/>
					<KpiCard
						label="Total Tokens"
						value={fmtNum(data.total_tokens)}
						trend={data.tokens_trend_pct}
						sparkline={data.tokens_sparkline}
						href="/orgs/{slug}/analytics/tokens"
						color="#f59e0b"
						tooltip="Total tokens processed across all sessions, including input, output, and cached tokens."
					/>
				</div>
				<SessionQualityBar
					avgDurationMs={data.avg_session_duration_ms}
					avgToolCalls={data.avg_tool_calls_per_session}
					avgCompactions={data.avg_compactions_per_session}
				/>
			</div>

			<!-- Right: Compliance + Cache savings -->
			<div class="flex-1 space-y-4">
				<ComplianceCard
					score={data.compliance_score_pct}
					trend={data.compliance_trend_pct}
					unsignedSessions={data.unsigned_sessions}
					chainVerified={data.chain_verified}
					href="/orgs/{slug}/compliance"
				/>
				<CacheSavingsCard
					savingsUsd={data.cache_savings_usd}
					savingsPct={data.cache_savings_pct}
					href="/orgs/{slug}/analytics/tokens"
				/>
			</div>
		</div>
	{/if}
</div>
