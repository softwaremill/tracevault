export function useDialog() {
	let open = $state(false);
	let loading = $state(false);
	let error = $state('');

	return {
		get open() {
			return open;
		},
		get loading() {
			return loading;
		},
		get error() {
			return error;
		},
		show() {
			open = true;
			error = '';
		},
		hide() {
			open = false;
			error = '';
			loading = false;
		},
		setLoading(v: boolean) {
			loading = v;
		},
		setError(msg: string) {
			error = msg;
		}
	};
}
