<script lang="ts">
	import { page } from '$app/stores';
	import { goto } from '$app/navigation';
	import { useFetch } from '$lib/hooks/use-fetch.svelte';
	import { fmtNum, fmtCost } from '$lib/utils/format';
	import PeriodSwitcher from '$lib/components/dashboard/PeriodSwitcher.svelte';
	import KpiCard from '$lib/components/dashboard/KpiCard.svelte';
	import SessionQualityBar from '$lib/components/dashboard/SessionQualityBar.svelte';
	import ComplianceCard from '$lib/components/dashboard/ComplianceCard.svelte';
	import CacheSavingsCard from '$lib/components/dashboard/CacheSavingsCard.svelte';
	import LoadingState from '$lib/components/LoadingState.svelte';
	import ErrorState from '$lib/components/ErrorState.svelte';
	import * as Table from '$lib/components/ui/table/index.js';

	interface TopAuthor {
		author: string;
		sessions: number;
		tokens: number;
		cost: number;
	}

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
		top_authors: TopAuthor[];
	}

	const slug = $derived($page.params.slug);

	let period = $state<'7d' | '30d' | 'month'>('7d');

	const dashboard = useFetch<DashboardData>(
		() => `/api/v1/orgs/${slug}/dashboard?period=${period}`
	);
</script>

<div class="space-y-4">
	<div class="flex items-center justify-between">
		<h1 class="text-xl font-semibold">Dashboard</h1>
		<PeriodSwitcher value={period} onchange={(p) => (period = p)} />
	</div>

	{#if dashboard.loading && !dashboard.data}
		<LoadingState />
	{:else if dashboard.error}
		<ErrorState message={dashboard.error} onRetry={dashboard.refetch} />
	{:else if dashboard.data}
		{@const data = dashboard.data}
		<!-- Top Authors Leaderboard -->
		{#if data.top_authors.length > 0}
			<div class="border-border overflow-hidden rounded-lg border">
				<div class="border-border flex items-center justify-between border-b px-3 py-2">
					<h3 class="text-sm font-semibold">Top Authors by Cost</h3>
					<a href="/orgs/{slug}/analytics/authors" class="text-muted-foreground hover:text-foreground text-xs transition-colors">View all &rarr;</a>
				</div>
				<Table.Root class="text-xs">
					<Table.Header>
						<Table.Row class="bg-muted/30 border-border border-b">
							<Table.Head class="text-muted-foreground text-[10px] font-semibold uppercase tracking-wider">#</Table.Head>
							<Table.Head class="text-muted-foreground text-[10px] font-semibold uppercase tracking-wider">Author</Table.Head>
							<Table.Head class="text-muted-foreground text-[10px] font-semibold uppercase tracking-wider text-right">Sessions</Table.Head>
							<Table.Head class="text-muted-foreground text-[10px] font-semibold uppercase tracking-wider text-right">Tokens</Table.Head>
							<Table.Head class="text-muted-foreground text-[10px] font-semibold uppercase tracking-wider text-right">Cost</Table.Head>
						</Table.Row>
					</Table.Header>
					<Table.Body>
						{#each data.top_authors as author, i}
							<Table.Row class="cursor-pointer border-l-[3px] border-l-transparent transition-colors hover:border-l-primary" onclick={() => goto(`/orgs/${slug}/analytics/authors`)}>
								<Table.Cell class="text-muted-foreground font-mono">{i + 1}</Table.Cell>
								<Table.Cell class="font-medium">{author.author}</Table.Cell>
								<Table.Cell class="text-right font-mono">{fmtNum(author.sessions)}</Table.Cell>
								<Table.Cell class="text-right font-mono">{fmtNum(author.tokens)}</Table.Cell>
								<Table.Cell class="text-right font-mono">{fmtCost(author.cost)}</Table.Cell>
							</Table.Row>
						{/each}
					</Table.Body>
				</Table.Root>
			</div>
		{/if}

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
