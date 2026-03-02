<script lang="ts">
	import { page } from '$app/stores';
	import { api } from '$lib/api';
	import * as Card from '$lib/components/ui/card/index.js';
	import * as Table from '$lib/components/ui/table/index.js';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import Chart from '$lib/components/chart.svelte';
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

	async function fetchData(search: string) {
		loading = true;
		error = '';
		try {
			data = await api.get<SessionsResponse>('/api/v1/analytics/sessions' + (search ? '?' + search : ''));
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
		<p class="text-muted-foreground">Loading...</p>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if data}
		<div class="grid grid-cols-1 gap-4 md:grid-cols-3">
			<Card.Root>
				<Card.Header class="pb-2">
					<Card.Description>Total Sessions</Card.Description>
				</Card.Header>
				<Card.Content>
					<p class="text-2xl font-bold">{fmtNum(data.total_sessions)}</p>
				</Card.Content>
			</Card.Root>
			<Card.Root>
				<Card.Header class="pb-2">
					<Card.Description>Avg Duration</Card.Description>
				</Card.Header>
				<Card.Content>
					<p class="text-2xl font-bold">{fmtDuration(data.avg_duration_ms)}</p>
				</Card.Content>
			</Card.Root>
			<Card.Root>
				<Card.Header class="pb-2">
					<Card.Description>Avg Messages/Session</Card.Description>
				</Card.Header>
				<Card.Content>
					<p class="text-2xl font-bold">
						{data.avg_messages_per_session != null ? data.avg_messages_per_session.toFixed(1) : '-'}
					</p>
				</Card.Content>
			</Card.Root>
		</div>

		<Card.Root>
			<Card.Header>
				<Card.Title>Tool Frequency</Card.Title>
			</Card.Header>
			<Card.Content class="flex justify-center">
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
			</Card.Content>
		</Card.Root>

		<Card.Root>
			<Card.Header>
				<Card.Title>Sessions</Card.Title>
			</Card.Header>
			<Card.Content>
				{#if data.sessions.length > 0}
					<Table.Root>
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
								<Table.Row>
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
											<Badge variant="outline">{session.model}</Badge>
										{:else}
											<span class="text-muted-foreground">-</span>
										{/if}
									</Table.Cell>
									<Table.Cell class="text-sm">{fmtRelativeTime(session.started_at)}</Table.Cell>
								</Table.Row>
							{/each}
						</Table.Body>
					</Table.Root>
				{:else}
					<p class="text-muted-foreground text-sm">No sessions</p>
				{/if}
			</Card.Content>
		</Card.Root>
	{/if}
</div>
