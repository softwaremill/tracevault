export interface BranchInfo {
	name: string;
	type: 'branch' | 'tag';
	is_default: boolean;
}

export interface TreeEntry {
	name: string;
	type: 'tree' | 'blob';
	size: number | null;
	path: string;
}

export interface BlobResponse {
	content: string | null;
	size: number;
	language: string | null;
	truncated: boolean;
	path: string;
}

export interface BlameHunk {
	commit_sha: string;
	author: string;
	date: string;
	start_line: number;
	end_line: number;
	message: string | null;
}

export interface StoryResponse {
	story: string;
	function_name: string;
	kind: string;
	line_range: [number, number];
	commits_analyzed: string[];
	sessions_referenced: string[];
	cached: boolean;
	generated_at: string;
}

export interface FileCommit {
	sha: string;
	message: string;
	author: string;
	date: string;
}

export interface RefInfo {
	sha: string;
	message: string;
	author: string;
	date: string;
}
