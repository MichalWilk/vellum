import type { PageLoad } from './$types';
import { getGraph } from '$lib/api';

export const load: PageLoad = async ({ fetch, params }) => {
	const graph = await getGraph(fetch, params.vault);
	return { graph, vault: params.vault };
};
