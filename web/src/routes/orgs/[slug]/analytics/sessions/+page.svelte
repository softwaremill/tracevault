<script lang="ts">
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import * as Table from '$lib/components/ui/table/index.js';
	import Chart from '$lib/components/chart.svelte';
	import SessionDetailPanel from '$lib/components/session-detail/SessionDetailPanel.svelte';
	import {
		Chart as ChartJS,
		CategoryScale,
		LinearScale,
		PointElement,
		LineElement,
		BarElement,
		ArcElement,
		Title,
		Tooltip,
		Legend,
		Filler
	} from 'chart.js';

	ChartJS.register(
		CategoryScale,
		LinearScale,
		PointElement,
		LineElement,
		BarElement,
		ArcElement,
		Title,
		Tooltip,
		Legend,
		Filler
	);

	const COLORS = ['#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899', '#06b6d4', '#84cc16'];

	interface SessionItem {
		id: string;
		session_id: string;
		model: string | null;
		duration_ms: number | null;
		started_at: string | null;
		ended_at: string | null;
		user_messages: number | null;
		assistant_messages: number | null;
		tool_calls: Record<string, number> | null;
		total_tool_calls: number | null;
		total_tokens: number | null;
		estimated_cost_usd: number | null;
		compactions: number | null;
		commit_sha: string;
		author: string;
		repo_name: string;
	}

	interface SessionsResponse {
		sessions: SessionItem[];
		tool_frequency: Record<string, number>;
		avg_duration_ms: number | null;
		avg_messages_per_session: number | null;
		total_sessions: number;
	}

	let data: SessionsResponse | null = $state(null);
	let loading = $state(true);
	let error = $state('');
	let sortCol = $state<string>('started_at');
	let sortDir = $state<'asc' | 'desc'>('desc');
	let expandedSessionId = $state<string | null>(null);

	function toggleExpand(id: string) {
		expandedSessionId = expandedSessionId === id ? null : id;
	}

	const slug = $derived($page.params.slug);

	async function fetchData(search: string) {
		loading = true;
		error = '';
		try {
			data = await api.get<SessionsResponse>(`/api/v1/orgs/${slug}/analytics/sessions` + (search ? '?' + search : ''));
		} catch (err) {
			error = err instanceof Error ? err.message : 'Failed to load';
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		const search = $page.url.search.replace(/^\?/, '');
		fetchData(search);
	});

	function fmtDuration(ms: number | null): string {
		if (ms == null) return '-';
		const totalSeconds = Math.floor(ms / 1000);
		const hours = Math.floor(totalSeconds / 3600);
		const minutes = Math.floor((totalSeconds % 3600) / 60);
		const seconds = totalSeconds % 60;
		if (hours >= 1) return `${hours}h ${minutes}m`;
		if (minutes >= 1) return `${minutes}m ${seconds}s`;
		return `${seconds}s`;
	}

	function fmtCost(n: number | null): string {
		if (n == null) return '-';
		return `$${n.toFixed(2)}`;
	}

	function fmtNum(n: number): string {
		if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
		if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
		return String(n);
	}

	function fmtRelativeTime(iso: string | null): string {
		if (!iso) return '-';
		const diff = Date.now() - new Date(iso).getTime();
		const minutes = Math.floor(diff / 60000);
		const hours = Math.floor(minutes / 60);
		const days = Math.floor(hours / 24);
		if (days > 0) return `${days}d ago`;
		if (hours > 0) return `${hours}h ago`;
		if (minutes > 0) return `${minutes}m ago`;
		return 'just now';
	}

	function sortBy(col: string) {
		if (sortCol === col) {
			sortDir = sortDir === 'asc' ? 'desc' : 'asc';
		} else {
			sortCol = col;
			sortDir = 'desc';
		}
	}

	function sortedSessions(sessions: SessionItem[]): SessionItem[] {
		return [...sessions].sort((a, b) => {
			let av: number | string | null;
			let bv: number | string | null;
			switch (sortCol) {
				case 'duration_ms': av = a.duration_ms; bv = b.duration_ms; break;
				case 'messages': av = (a.user_messages ?? 0) + (a.assistant_messages ?? 0); bv = (b.user_messages ?? 0) + (b.assistant_messages ?? 0); break;
				case 'total_tool_calls': av = a.total_tool_calls; bv = b.total_tool_calls; break;
				case 'estimated_cost_usd': av = a.estimated_cost_usd; bv = b.estimated_cost_usd; break;
				case 'started_at': av = a.started_at ?? ''; bv = b.started_at ?? ''; break;
				case 'repo_name': av = a.repo_name; bv = b.repo_name; break;
				case 'author': av = a.author; bv = b.author; break;
				default: av = a.started_at ?? ''; bv = b.started_at ?? '';
			}
			if (av == null && bv == null) return 0;
			if (av == null) return 1;
			if (bv == null) return -1;
			const diff = typeof av === 'string' ? av.localeCompare(bv as string) : (av as number) - (bv as number);
			return sortDir === 'asc' ? diff : -diff;
		});
	}

	function sortIndicator(col: string): string {
		if (sortCol !== col) return '';
		return sortDir === 'asc' ? ' \u2191' : ' \u2193';
	}

	function toolFrequencyChartData(d: SessionsResponse) {
		const entries = Object.entries(d.tool_frequency).sort((a, b) => b[1] - a[1]).slice(0, 10);
		return {
			labels: entries.map(([k]) => k),
			datasets: [
				{
					data: entries.map(([, v]) => v),
					backgroundColor: COLORS.slice(0, entries.length).concat(
						Array(Math.max(0, entries.length - COLORS.length)).fill('#94a3b8')
					)
				}
			]
		};
	}
</script>

<svelte:head>
	<title>Session Analytics - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	<h1 class="text-2xl font-bold">Session Analytics</h1>

	{#if loading}
		<div class="text-muted-foreground flex items-center justify-center gap-2 py-12 text-sm">
			<span class="inline-block h-4 w-4 animate-spin rounded-full border-2 border-current border-t-transparent"></span>
			Loading...
		</div>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if data}
		<!-- Stat cards -->
		<div class="border-border overflow-hidden rounded-lg border">
			<div class="grid grid-cols-2 gap-px md:grid-cols-3">
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Total Sessions</div>
					<div class="mt-1 text-lg font-semibold">{fmtNum(data.total_sessions)}</div>
				</div>
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Avg Duration</div>
					<div class="mt-1 text-lg font-semibold">{fmtDuration(data.avg_duration_ms)}</div>
				</div>
				<div class="bg-background p-3">
					<div class="text-muted-foreground text-[11px] uppercase tracking-wide">Avg Messages/Session</div>
					<div class="mt-1 text-lg font-semibold">
						{data.avg_messages_per_session != null ? data.avg_messages_per_session.toFixed(1) : '-'}
					</div>
				</div>
			</div>
		</div>

		<!-- Tool Frequency chart -->
		<div class="border-border rounded-lg border p-3">
			<h4 class="mb-2 text-sm font-semibold">Tool Frequency</h4>
			<div class="flex justify-center">
				{#if Object.keys(data.tool_frequency).length > 0}
					<div class="max-w-[400px]">
						<Chart
							type="doughnut"
							data={toolFrequencyChartData(data)}
							options={{ responsive: true, plugins: { legend: { position: 'bottom' } } }}
						/>
					</div>
				{:else}
					<p class="text-muted-foreground text-sm">No tool data</p>
				{/if}
			</div>
		</div>

		<!-- Sessions table -->
		<div class="border-border overflow-hidden rounded-lg border">
			<h2 class="text-sm font-semibold uppercase tracking-wide text-muted-foreground px-3 pt-3 pb-2">Sessions</h2>
			{#if data.sessions.length > 0}
				<Table.Root class="text-xs">
					<Table.Header>
						<Table.Row>
							<Table.Head>Session ID</Table.Head>
							<Table.Head>
								<button class="hover:underline" onclick={() => sortBy('repo_name')}>
									Repo{sortIndicator('repo_name')}
								</button>
							</Table.Head>
							<Table.Head>
								<button class="hover:underline" onclick={() => sortBy('author')}>
									Author{sortIndicator('author')}
								</button>
							</Table.Head>
							<Table.Head>
								<button class="hover:underline" onclick={() => sortBy('duration_ms')}>
									Duration{sortIndicator('duration_ms')}
								</button>
							</Table.Head>
							<Table.Head>
								<button class="hover:underline" onclick={() => sortBy('messages')}>
									Messages{sortIndicator('messages')}
								</button>
							</Table.Head>
							<Table.Head>
								<button class="hover:underline" onclick={() => sortBy('total_tool_calls')}>
									Tool Calls{sortIndicator('total_tool_calls')}
								</button>
							</Table.Head>
							<Table.Head>
								<button class="hover:underline" onclick={() => sortBy('estimated_cost_usd')}>
									Cost{sortIndicator('estimated_cost_usd')}
								</button>
							</Table.Head>
							<Table.Head>Model</Table.Head>
							<Table.Head>
								<button class="hover:underline" onclick={() => sortBy('started_at')}>
									Started{sortIndicator('started_at')}
								</button>
							</Table.Head>
						</Table.Row>
					</Table.Header>
					<Table.Body>
						{#each sortedSessions(data.sessions) as session}
							<Table.Row
								class="cursor-pointer hover:bg-muted/40 transition-colors"
								onclick={() => toggleExpand(session.id)}
							>
								<Table.Cell class="font-mono text-sm">{session.session_id.slice(0, 8)}</Table.Cell>
								<Table.Cell>{session.repo_name}</Table.Cell>
								<Table.Cell>{session.author}</Table.Cell>
								<Table.Cell class="font-mono text-sm">{fmtDuration(session.duration_ms)}</Table.Cell>
								<Table.Cell class="font-mono text-sm">
									{(session.user_messages ?? 0) + (session.assistant_messages ?? 0)}
								</Table.Cell>
								<Table.Cell class="font-mono text-sm">{session.total_tool_calls ?? 0}</Table.Cell>
								<Table.Cell class="font-mono text-sm">{fmtCost(session.estimated_cost_usd)}</Table.Cell>
								<Table.Cell>
									{#if session.model}
										<span class="rounded-full px-2 py-0.5 text-[10px]" style="background: rgba(79,110,247,0.12); color: #4f6ef7; border: 1px solid rgba(79,110,247,0.25)">{session.model}</span>
									{:else}
										<span class="text-muted-foreground">-</span>
									{/if}
								</Table.Cell>
								<Table.Cell class="text-sm">{fmtRelativeTime(session.started_at)}</Table.Cell>
							</Table.Row>
							{#if expandedSessionId === session.id}
								<Table.Row>
									<Table.Cell colspan={9} class="p-0">
										<SessionDetailPanel sessionId={session.id} />
									</Table.Cell>
								</Table.Row>
							{/if}
						{/each}
					</Table.Body>
				</Table.Root>
			{:else}
				<p class="text-muted-foreground px-3 pb-3 text-sm">No sessions</p>
			{/if}
		</div>
	{/if}
</div>
