import { writable } from 'svelte/store';
import { browser } from '$app/environment';
import { api } from '$lib/api';

interface User {
	user_id: string;
	email: string;
	name: string | null;
}

interface AuthState {
	user: User | null;
	isAuthenticated: boolean;
	loading: boolean;
}

function createAuthStore() {
	const { subscribe, set } = writable<AuthState>({
		user: null,
		isAuthenticated: false,
		loading: true
	});

	return {
		subscribe,
		async init() {
			if (!browser) return;
			const token = localStorage.getItem('tracevault_token');
			if (!token) {
				set({ user: null, isAuthenticated: false, loading: false });
				return;
			}
			try {
				const user = await api.get<User>('/api/v1/auth/me');
				set({ user, isAuthenticated: true, loading: false });
			} catch {
				localStorage.removeItem('tracevault_token');
				set({ user: null, isAuthenticated: false, loading: false });
			}
		},
		setToken(token: string) {
			if (browser) localStorage.setItem('tracevault_token', token);
		},
		logout() {
			if (browser) localStorage.removeItem('tracevault_token');
			set({ user: null, isAuthenticated: false, loading: false });
		}
	};
}

export const auth = createAuthStore();
