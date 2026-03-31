import { writable } from 'svelte/store';
import { browser } from '$app/environment';

const STORAGE_KEY = 'sidebar-expanded';

function createSidebarStore() {
	const initial = browser ? localStorage.getItem(STORAGE_KEY) === 'true' : false;
	const { subscribe, update } = writable(initial);

	return {
		subscribe,
		toggle() {
			update((v) => {
				const next = !v;
				if (browser) localStorage.setItem(STORAGE_KEY, String(next));
				return next;
			});
		}
	};
}

export const sidebarExpanded = createSidebarStore();
