import type { LayoutLoad } from './$types';
import { getMe } from '$lib/api';

export const load: LayoutLoad = async ({ fetch }) => {
	const user = await getMe(fetch);
	return { user };
};
