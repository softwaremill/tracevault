export type SessionDisplayStatus = 'active' | 'completed' | 'stale';

export function sessionStatus(
	status: string,
	updatedAt: string | null
): SessionDisplayStatus {
	if (status === 'completed') return 'completed';
	if (status === 'active' && updatedAt) {
		const diff = Date.now() - new Date(updatedAt).getTime();
		if (diff > 30 * 60 * 1000) return 'stale';
	}
	return 'active';
}

export const statusStyles: Record<
	SessionDisplayStatus,
	{ bg: string; text: string; label: string }
> = {
	active: {
		bg: 'bg-green-500/15',
		text: 'text-green-600 dark:text-green-400',
		label: 'Active'
	},
	completed: {
		bg: 'bg-zinc-500/15',
		text: 'text-zinc-500 dark:text-zinc-400',
		label: 'Completed'
	},
	stale: {
		bg: 'bg-yellow-500/15',
		text: 'text-yellow-600 dark:text-yellow-400',
		label: 'Stale'
	}
};
