export interface Paginated<T> {
	items: T[];
	next_cursor: string | null;
	total_count: number;
}

export type Period = '7d' | '30d' | 'month';

export interface ApiError {
	error: string;
}
