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
		Title,
		Tooltip,
		Legend,
		Filler
	);

	const COLORS = ['#3b82f6', '#10b981', '#f59e0b', '#ef4444', '#8b5cf6', '#ec4899', '#06b6d4', '#84cc16'];

	interface AuthorLeaderboard {
		author: string;
		commits: number;
		sessions: number;
		tokens: number;
		cost: number;
		ai_pct: number | null;
		last_active: string;
		avg_duration_ms: number | null;
		total_tool_calls: number;
		total_compactions: number;
	}
	interface AuthorTimeline {
		date: string;
		author: string;
		commits: number;
	}
	interface AuthorModelPreference {
		author: string;
		model: string;
		sessions: number;
	}
	interface AuthorsResponse {
		leaderboard: AuthorLeaderboard[];
		timeline: AuthorTimeline[];
		model_preferences: AuthorModelPreference[];
	}

	let data: AuthorsResponse | null = $state(null);
	let loading = $state(true);
	let error = $state('');

	async function fetchData(search: string) {
		loading = true;
		error = '';
		try {
			data = await api.get<AuthorsResponse>('/api/v1/analytics/authors' + (search ? '?' + search : ''));
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

	function fmtNum(n: number): string {
		if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
		if (n >= 1_000) return `${(n / 1_000).toFixed(1)}k`;
		return String(n);
	}

	function fmtDate(iso: string): string {
		return new Date(iso).toLocaleDateString();
	}

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

	function timelineChartData(d: AuthorsResponse) {
		const authors = [...new Set(d.timeline.map((t) => t.author))];
		const dates = [...new Set(d.timeline.map((t) => t.date))].sort();
		return {
			labels: dates,
			datasets: authors.map((author, i) => ({
				label: author,
				data: dates.map((date) => d.timeline.find((t) => t.date === date && t.author === author)?.commits ?? 0),
				borderColor: COLORS[i % COLORS.length],
				backgroundColor: COLORS[i % COLORS.length] + '33',
				tension: 0.3
			}))
		};
	}

	function modelPrefsForAuthor(author: string): AuthorModelPreference[] {
		if (!data) return [];
		return data.model_preferences.filter((p) => p.author === author);
	}
</script>

<svelte:head>
	<title>Author Analytics - TraceVault</title>
</svelte:head>

<div class="space-y-6">
	<h1 class="text-2xl font-bold">Author Analytics</h1>

	{#if loading}
		<p class="text-muted-foreground">Loading...</p>
	{:else if error}
		<p class="text-destructive">{error}</p>
	{:else if data}
		<Card.Root>
			<Card.Header>
				<Card.Title>Leaderboard</Card.Title>
			</Card.Header>
			<Card.Content>
				{#if data.leaderboard.length > 0}
					<Table.Root>
						<Table.Header>
							<Table.Row>
								<Table.Head>Author</Table.Head>
								<Table.Head>Commits</Table.Head>
								<Table.Head>Sessions</Table.Head>
								<Table.Head>Tokens</Table.Head>
								<Table.Head>Cost</Table.Head>
								<Table.Head>AI %</Table.Head>
								<Table.Head>Avg Duration</Table.Head>
								<Table.Head>Tool Calls</Table.Head>
								<Table.Head>Compactions</Table.Head>
								<Table.Head>Last Active</Table.Head>
								<Table.Head>Models</Table.Head>
							</Table.Row>
						</Table.Header>
						<Table.Body>
							{#each data.leaderboard as row}
								<Table.Row>
									<Table.Cell class="font-medium">{row.author}</Table.Cell>
									<Table.Cell>{row.commits}</Table.Cell>
									<Table.Cell>{row.sessions}</Table.Cell>
									<Table.Cell class="font-mono text-sm">{fmtNum(row.tokens)}</Table.Cell>
									<Table.Cell class="font-mono text-sm">${row.cost.toFixed(2)}</Table.Cell>
									<Table.Cell>
										{row.ai_pct != null ? `${row.ai_pct.toFixed(1)}%` : 'N/A'}
									</Table.Cell>
									<Table.Cell class="font-mono text-sm">{fmtDuration(row.avg_duration_ms)}</Table.Cell>
									<Table.Cell class="font-mono text-sm">{fmtNum(row.total_tool_calls)}</Table.Cell>
									<Table.Cell>{row.total_compactions}</Table.Cell>
									<Table.Cell>{fmtDate(row.last_active)}</Table.Cell>
									<Table.Cell>
										<div class="flex flex-wrap gap-1">
											{#each modelPrefsForAuthor(row.author) as pref}
												<Badge variant="outline">{pref.model} ({pref.sessions})</Badge>
											{/each}
										</div>
									</Table.Cell>
								</Table.Row>
							{/each}
						</Table.Body>
					</Table.Root>
				{:else}
					<p class="text-muted-foreground text-sm">No data</p>
				{/if}
			</Card.Content>
		</Card.Root>

		<Card.Root>
			<Card.Header>
				<Card.Title>Activity Timeline</Card.Title>
			</Card.Header>
			<Card.Content>
				{#if data.timeline.length > 0}
					<Chart
					type="line"
						data={timelineChartData(data)}
						options={{ responsive: true, plugins: { legend: { position: 'top' } } }}
					/>
				{:else}
					<p class="text-muted-foreground text-sm">No data</p>
				{/if}
			</Card.Content>
		</Card.Root>
	{/if}
</div>
