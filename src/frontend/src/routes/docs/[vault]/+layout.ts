import type { LayoutLoad } from './$types';
import { getTree } from '$lib/api';

export const load: LayoutLoad = async ({ fetch, params }) => {
	const tree = await getTree(fetch, params.vault);
	return { tree, vault: params.vault };
};
