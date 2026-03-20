import { writable } from 'svelte/store';

export const searchOpen = writable(false);
export const searchQuery = writable('');

export function toggleSearch() {
	searchOpen.update((v) => !v);
}

export function openSearchWithQuery(q: string) {
	searchQuery.set(q);
	searchOpen.set(true);
}
