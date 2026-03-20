<script lang="ts">
	import '../app.css';
	import type { LayoutData } from './$types';
	import type { Snippet } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { searchOpen, searchQuery, toggleSearch } from '$lib/search';
	import { searchDocs } from '$lib/api';
	import type { SearchResult } from '$lib/types';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Command from '$lib/components/ui/command/index.js';
	import ThemeToggle from '$lib/components/theme-toggle.svelte';
	import { Network, Search, LogIn, LogOut, Library } from '@lucide/svelte';

	let { data, children }: { data: LayoutData; children: Snippet } = $props();

	let query = $state('');
	let results = $state<SearchResult[]>([]);

	let vault = $derived((() => {
		const match = page.url.pathname.match(/^\/(docs|graph)\/([^/]+)/);
		return match ? match[2] : null;
	})());

	$effect(() => {
		if (!$searchOpen) {
			query = '';
			results = [];
			searchQuery.set('');
			return;
		}
		const prefill = $searchQuery;
		if (prefill) {
			query = prefill;
			searchQuery.set('');
		}
	});

	$effect(() => {
		const q = query.trim();
		if (!q) {
			results = [];
			return;
		}
		const searchVault = vault ?? 'docs';
		const timer = setTimeout(async () => {
			results = await searchDocs(fetch, q, searchVault);
		}, 300);
		return () => clearTimeout(timer);
	});

	function onKeydown(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
			e.preventDefault();
			toggleSearch();
		}
	}

	function sanitizeSnippet(html: string): string {
		let safe = html.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
		safe = safe.replace(/&lt;b&gt;/g, '<b>').replace(/&lt;\/b&gt;/g, '</b>');
		return safe;
	}

	function selectResult(result: SearchResult) {
		searchOpen.set(false);
		const targetVault = vault ?? 'docs';
		goto(`/docs/${targetVault}/${result.path}`);
	}
</script>

<svelte:window onkeydown={onKeydown} />

<svelte:head>
	<title>Vellum</title>
</svelte:head>

<div class="bg-background text-foreground min-h-screen">
	<nav
		class="bg-background/95 supports-[backdrop-filter]:bg-background/60 sticky top-0 z-10 flex items-center justify-between border-b px-6 py-3 backdrop-blur"
	>
		<div class="flex items-center gap-2">
			<a href="/" class="text-foreground flex items-center gap-1.5 text-lg font-semibold">
				<img src="/favicon.svg" alt="Vellum" class="h-5 w-5" />
				Vellum
			</a>
			{#if vault}
				<span class="text-muted-foreground text-sm">/</span>
				<a href="/docs/{vault}" class="text-muted-foreground hover:text-foreground flex items-center gap-1 text-sm font-medium">
					<Library class="h-3.5 w-3.5" />
					{vault}
				</a>
			{/if}
			{#if vault}
				<Button variant="ghost" size="sm" href="/graph/{vault}" class="gap-1">
					<Network class="h-3.5 w-3.5" />
					Graph
				</Button>
			{:else}
				<Button variant="ghost" size="sm" href="/graph" class="gap-1">
					<Network class="h-3.5 w-3.5" />
					Graph
				</Button>
			{/if}
		</div>
		<div class="flex items-center gap-2">
			<Button variant="outline" size="sm" onclick={toggleSearch} class="text-muted-foreground gap-2">
				<Search class="h-3.5 w-3.5" />
				Search
				<kbd class="bg-muted text-muted-foreground rounded px-1.5 py-0.5 text-[10px] font-mono">&#8984;K</kbd>
			</Button>
			<ThemeToggle />
			{#if data.user}
				<span class="text-muted-foreground text-sm">{data.user.name}</span>
				{#if data.user.sub !== 'anonymous'}
					<Button variant="ghost" size="sm" href="/api/auth/logout" class="gap-1">
						<LogOut class="h-3.5 w-3.5" />
						Logout
					</Button>
				{/if}
			{:else}
				<Button variant="default" size="sm" href="/api/auth/login?return_to={encodeURIComponent(page.url.pathname)}" class="gap-1">
					<LogIn class="h-3.5 w-3.5" />
					Login
				</Button>
			{/if}
		</div>
	</nav>
	{@render children()}
</div>

<Command.Dialog bind:open={$searchOpen} title="Search documents" description="Search for documents in your vault" shouldFilter={false}>
	<Command.Input placeholder="Search documents..." bind:value={query} />
	<Command.List>
		{#if query.trim().length > 0 && results.length === 0}
			<Command.Empty>No results found.</Command.Empty>
		{/if}
		{#if results.length > 0}
			<Command.Group heading="Results">
				{#each results as result (result.path)}
					<Command.Item onSelect={() => selectResult(result)}>
						<div class="flex flex-col gap-0.5">
							<span class="text-sm font-medium">{result.title || result.path}</span>
							<span class="text-muted-foreground text-xs">{result.path}</span>
							{#if result.snippet}
								<span class="text-muted-foreground mt-0.5 text-xs [&_mark]:bg-primary/20 [&_mark]:font-medium">
									{@html sanitizeSnippet(result.snippet)}
								</span>
							{/if}
						</div>
					</Command.Item>
				{/each}
			</Command.Group>
		{/if}
	</Command.List>
</Command.Dialog>
