import { api } from '$lib/api';

interface UseFetchOptions<T> {
	initial?: T;
	immediate?: boolean;
}

export function useFetch<T>(urlFn: () => string, opts?: UseFetchOptions<T>) {
	let data = $state<T>(opts?.initial as T);
	let loading = $state(true);
	let error = $state('');

	async function refetch() {
		loading = true;
		error = '';
		try {
			data = await api.get<T>(urlFn());
		} catch (err: unknown) {
			error = err instanceof Error ? err.message : String(err);
		} finally {
			loading = false;
		}
	}

	if (opts?.immediate !== false) {
		$effect(() => {
			urlFn(); // track reactive dependencies
			refetch();
		});
	}

	return {
		get data() {
			return data;
		},
		get loading() {
			return loading;
		},
		get error() {
			return error;
		},
		refetch
	};
}
