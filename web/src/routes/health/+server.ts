import type { RequestHandler } from './$types';

const API_SERVER = import.meta.env.PUBLIC_API_URL || 'http://localhost:3000';

export const GET: RequestHandler = async () => {
	return fetch(`${API_SERVER}/health`);
};
