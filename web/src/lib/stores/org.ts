import { writable } from 'svelte/store';
import { api } from '$lib/api';

interface OrgMembership {
	org_id: string;
	org_name: string;
	display_name: string | null;
	role: string;
}

interface OrgState {
	current: OrgMembership | null;
	all: OrgMembership[];
	loading: boolean;
}

function createOrgStore() {
	const { subscribe, set, update } = writable<OrgState>({
		current: null,
		all: [],
		loading: true
	});

	return {
		subscribe,
		async loadOrgs() {
			try {
				const orgs = await api.get<OrgMembership[]>('/api/v1/me/orgs');
				update((s) => ({ ...s, all: orgs, loading: false }));
				return orgs;
			} catch {
				set({ current: null, all: [], loading: false });
				return [];
			}
		},
		setCurrent(org: OrgMembership) {
			update((s) => ({ ...s, current: org }));
		},
		clear() {
			set({ current: null, all: [], loading: true });
		}
	};
}

export const orgStore = createOrgStore();
