export interface User {
	sub: string;
	name: string;
	email: string;
	roles: string[];
}

export interface FileNode {
	name: string;
	path: string;
	type: 'file' | 'dir';
	children?: FileNode[];
}

export interface DocResponse {
	content: string;
	frontmatter: Record<string, unknown>;
	path: string;
	last_modified: string | null;
	last_modified_by: string | null;
}

export interface GraphNode {
	id: string;
	label: string;
	node_type: string;
}

export interface GraphEdge {
	source: string;
	target: string;
}

export interface GraphData {
	nodes: GraphNode[];
	edges: GraphEdge[];
}

export type SearchType = 'content' | 'files' | 'tags' | 'headings';

export interface SearchResult {
	path: string;
	title: string;
	snippet: string;
	score: number;
	result_type: string;
	anchor?: string;
	tag?: string;
	doc_count?: number;
	heading_level?: number;
}

export interface VaultInfo {
	name: string;
	description: string | null;
}
