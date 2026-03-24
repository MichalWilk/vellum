import type { User, FileNode, DocResponse, GraphData, SearchResult, VaultInfo, SearchType } from './types';

type Fetch = typeof fetch;

export async function getMe(f: Fetch): Promise<User | null> {
	const res = await f('/api/me');
	if (!res.ok) return null;
	return res.json();
}

export async function getVaults(f: Fetch): Promise<VaultInfo[]> {
	const res = await f('/api/vaults');
	if (!res.ok) return [];
	return res.json();
}

export async function getTree(f: Fetch, vault: string): Promise<FileNode[]> {
	const res = await f(`/api/tree/${encodeURIComponent(vault)}`);
	if (!res.ok) return [];
	return res.json();
}

export async function getDoc(
	f: Fetch,
	vault: string,
	path: string
): Promise<{ doc: DocResponse } | { error: 'not_found' | 'forbidden' }> {
	const encodedPath = path.split('/').map(encodeURIComponent).join('/');
	const res = await f(`/api/doc/${encodeURIComponent(vault)}/${encodedPath}`);
	if (res.status === 403) return { error: 'forbidden' };
	if (!res.ok) return { error: 'not_found' };
	return { doc: await res.json() };
}

export async function getGraph(f: Fetch, vault: string): Promise<GraphData> {
	const res = await f(`/api/graph/${encodeURIComponent(vault)}`);
	if (!res.ok) return { nodes: [], edges: [] };
	return res.json();
}

export async function searchDocs(
	f: Fetch,
	query: string,
	vault: string,
	searchType: SearchType = 'content',
	limit = 20,
	tag?: string
): Promise<SearchResult[]> {
	const params = new URLSearchParams();
	if (query) params.set('q', query);
	params.set('vault', vault);
	params.set('limit', String(limit));
	params.set('type', searchType);
	if (tag) params.set('tag', tag);
	const res = await f(`/api/search?${params.toString()}`);
	if (!res.ok) return [];
	return res.json();
}
