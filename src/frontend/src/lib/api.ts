import type { User, FileNode, DocResponse, GraphData, SearchResult, VaultInfo } from './types';

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

export async function searchDocs(f: Fetch, query: string, vault: string, limit = 10): Promise<SearchResult[]> {
	const res = await f(
		`/api/search?q=${encodeURIComponent(query)}&vault=${encodeURIComponent(vault)}&limit=${limit}`
	);
	if (!res.ok) return [];
	return res.json();
}
