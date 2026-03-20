import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { getVaults } from '$lib/api';

export const load: PageLoad = async ({ fetch }) => {
	const vaults = await getVaults(fetch);
	if (vaults.length === 1) {
		throw redirect(302, `/docs/${vaults[0].name}/index.md`);
	}
	return { vaults };
};
