<script lang="ts">
	interface Props {
		password: string;
	}

	let { password }: Props = $props();

	interface Check {
		label: string;
		met: boolean;
	}

	const checks: Check[] = $derived([
		{ label: 'At least 10 characters', met: password.length >= 10 },
		{ label: 'Uppercase letter', met: /[A-Z]/.test(password) },
		{ label: 'Lowercase letter', met: /[a-z]/.test(password) },
		{ label: 'Number', met: /[0-9]/.test(password) },
		{ label: 'Special character', met: /[^A-Za-z0-9]/.test(password) }
	]);

	const score = $derived(checks.filter((c) => c.met).length);

	const strength = $derived(
		score <= 1 ? 'weak' : score <= 2 ? 'weak' : score <= 3 ? 'fair' : score <= 4 ? 'good' : 'strong'
	);

	const strengthLabel = $derived(
		password.length === 0 ? '' : strength === 'weak' ? 'Weak' : strength === 'fair' ? 'Fair' : strength === 'good' ? 'Good' : 'Strong'
	);

	const barColor = $derived(
		strength === 'weak'
			? 'bg-destructive'
			: strength === 'fair'
				? 'bg-orange-500'
				: strength === 'good'
					? 'bg-yellow-500'
					: 'bg-green-500'
	);

	const barWidth = $derived(password.length === 0 ? 0 : (score / 5) * 100);

	export function isStrong(): boolean {
		return score >= 4;
	}
</script>

{#if password.length > 0}
	<div class="space-y-2">
		<div class="flex items-center justify-between">
			<span class="text-xs text-muted-foreground">Password strength</span>
			<span
				class="text-xs font-medium {strength === 'weak'
					? 'text-destructive'
					: strength === 'fair'
						? 'text-orange-500'
						: strength === 'good'
							? 'text-yellow-600'
							: 'text-green-600'}">{strengthLabel}</span
			>
		</div>
		<div class="h-1.5 w-full rounded-full bg-muted">
			<div
				class="h-full rounded-full transition-all duration-300 {barColor}"
				style="width: {barWidth}%"
			></div>
		</div>
		<ul class="grid grid-cols-2 gap-x-4 gap-y-0.5">
			{#each checks as check}
				<li class="flex items-center gap-1.5 text-xs {check.met ? 'text-green-600' : 'text-muted-foreground'}">
					<span class="text-[10px]">{check.met ? '✓' : '○'}</span>
					{check.label}
				</li>
			{/each}
		</ul>
	</div>
{:else}
	<p class="text-xs text-muted-foreground">Must be strong: 10+ chars, mixed case, number, special character.</p>
{/if}
