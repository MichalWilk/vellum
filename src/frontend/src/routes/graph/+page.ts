import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { getVaults } from '$lib/api';

export const load: PageLoad = async ({ fetch }) => {
	const vaults = await getVaults(fetch);
	if (vaults.length > 0) {
		throw redirect(302, `/graph/${vaults[0].name}`);
	}
	throw redirect(302, '/');
};
