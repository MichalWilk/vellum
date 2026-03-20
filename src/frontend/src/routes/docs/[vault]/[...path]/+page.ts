import { error, redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types';
import { getDoc } from '$lib/api';

export const load: PageLoad = async ({ fetch, params, url }) => {
	const result = await getDoc(fetch, params.vault, params.path);
	if ('error' in result) {
		if (result.error === 'forbidden') {
			throw redirect(302, `/api/auth/login?return_to=${encodeURIComponent(url.pathname)}`);
		}
		throw error(404, 'Document not found');
	}
	return { doc: result.doc };
};
