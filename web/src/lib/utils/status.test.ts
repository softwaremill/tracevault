import { describe, it, expect } from 'vitest';
import { sessionStatus, statusStyles } from './status';

describe('sessionStatus', () => {
	it('returns completed for completed status', () => {
		expect(sessionStatus('completed', null)).toBe('completed');
	});

	it('returns active for recent active session', () => {
		const recent = new Date().toISOString();
		expect(sessionStatus('active', recent)).toBe('active');
	});

	it('returns stale for active session older than 30 minutes', () => {
		const old = new Date(Date.now() - 31 * 60 * 1000).toISOString();
		expect(sessionStatus('active', old)).toBe('stale');
	});

	it('returns active for active session without updatedAt', () => {
		expect(sessionStatus('active', null)).toBe('active');
	});

	it('returns active for active session within 30 minutes', () => {
		const recent = new Date(Date.now() - 10 * 60 * 1000).toISOString();
		expect(sessionStatus('active', recent)).toBe('active');
	});
});

describe('statusStyles', () => {
	it('has entries for all statuses', () => {
		expect(statusStyles.active.label).toBe('Active');
		expect(statusStyles.completed.label).toBe('Completed');
		expect(statusStyles.stale.label).toBe('Stale');
	});

	it('each entry has bg, text, and label', () => {
		for (const key of ['active', 'completed', 'stale'] as const) {
			expect(statusStyles[key]).toHaveProperty('bg');
			expect(statusStyles[key]).toHaveProperty('text');
			expect(statusStyles[key]).toHaveProperty('label');
		}
	});
});
