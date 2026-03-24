<script lang="ts">
	import '../app.css';
	import type { LayoutData } from './$types';
	import type { Snippet } from 'svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { searchOpen, searchQuery, toggleSearch } from '$lib/search';
	import { searchDocs } from '$lib/api';
	import type { SearchResult, SearchType } from '$lib/types';
	import { Button } from '$lib/components/ui/button/index.js';
	import * as Command from '$lib/components/ui/command/index.js';
	import ThemeToggle from '$lib/components/theme-toggle.svelte';
	import { Network, Search, LogIn, LogOut, Library, FileText, Hash } from '@lucide/svelte';

	let { data, children }: { data: LayoutData; children: Snippet } = $props();

	let query = $state('');
	let results = $state<SearchResult[]>([]);
	let searchType = $state<SearchType>(
		(typeof localStorage !== 'undefined' && localStorage.getItem('vellum-search-type') as SearchType) || 'content'
	);
	let selectedTag = $state<string | null>(null);

	const searchLabels: Record<SearchType, { placeholder: string; empty: string; heading: string }> = {
		content: { placeholder: 'Search content...', empty: 'No documents found.', heading: 'Results' },
		files: { placeholder: 'Search files...', empty: 'No files found.', heading: 'Files' },
		tags: { placeholder: 'Search tags...', empty: 'No tags found.', heading: 'Tags' },
		headings: { placeholder: 'Search headings...', empty: 'No headings found.', heading: 'Headings' },
	};

	let vault = $derived((() => {
		const match = page.url.pathname.match(/^\/(docs|graph)\/([^/]+)/);
		return match ? match[2] : null;
	})());

	$effect(() => {
		if (!$searchOpen) {
			query = '';
			results = [];
			selectedTag = null;
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
		const currentType = searchType;
		const currentTag = selectedTag;
		const searchVault = vault ?? 'docs';

		if (!q && !currentTag) {
			results = [];
			return;
		}

		if (currentType === 'files' && q.length < 2) {
			results = [];
			return;
		}

		const debounceMs = currentType === 'content' ? 300 : 150;
		const timer = setTimeout(async () => {
			results = await searchDocs(fetch, q, searchVault, currentType, 20, currentTag ?? undefined);
		}, debounceMs);
		return () => clearTimeout(timer);
	});

	function onKeydown(e: KeyboardEvent) {
		if ((e.metaKey || e.ctrlKey) && e.key === 'k') {
			e.preventDefault();
			toggleSearch();
		}
		if ($searchOpen && e.altKey) {
			const modes: SearchType[] = ['content', 'files', 'tags', 'headings'];
			const idx = parseInt(e.key) - 1;
			if (idx >= 0 && idx < modes.length) {
				e.preventDefault();
				setSearchType(modes[idx]);
			}
		}
	}

	function escapeHtml(s: string): string {
		return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;');
	}

	function sanitizeSnippet(html: string): string {
		let safe = escapeHtml(html);
		safe = safe.replace(/&lt;b&gt;/g, '<mark>').replace(/&lt;\/b&gt;/g, '</mark>');
		return safe;
	}

	function highlightMatch(text: string, q: string): string {
		const escaped = escapeHtml(text);
		if (!q) return escaped;
		const re = new RegExp(`(${q.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')})`, 'gi');
		return escaped.replace(re, '<mark>$1</mark>');
	}

	function setSearchType(t: SearchType) {
		searchType = t;
		selectedTag = null;
		query = '';
		results = [];
		if (typeof localStorage !== 'undefined') {
			localStorage.setItem('vellum-search-type', t);
		}
	}

	function selectTag(tagName: string) {
		selectedTag = tagName;
		query = '';
	}

	function clearTag() {
		selectedTag = null;
		query = '';
		results = [];
	}

	function selectResult(result: SearchResult) {
		if (result.result_type === 'tag' && result.tag) {
			selectTag(result.tag);
			return;
		}
		searchOpen.set(false);
		const targetVault = vault ?? 'docs';
		const anchor = result.anchor ? `#${result.anchor}` : '';
		goto(`/docs/${targetVault}/${result.path}${anchor}`);
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
	<div class="flex gap-1 border-b px-3 py-2">
		{#each [
			{ type: 'content', label: 'Content', key: '1' },
			{ type: 'files', label: 'Files', key: '2' },
			{ type: 'tags', label: 'Tags', key: '3' },
			{ type: 'headings', label: 'Headings', key: '4' }
		] as tab}
			<button
				tabindex={-1}
				class="rounded-md px-2.5 py-1 text-xs font-medium transition-colors {searchType === tab.type
					? 'bg-accent text-accent-foreground'
					: 'text-muted-foreground hover:text-foreground hover:bg-accent/50'}"
				onclick={() => setSearchType(tab.type as SearchType)}
			>
				{tab.label}
				<kbd class="ml-1 text-[9px] opacity-50">Alt+{tab.key}</kbd>
			</button>
		{/each}
	</div>
	{#if selectedTag}
		<div class="flex items-center gap-2 border-b px-3 py-2">
			<span class="bg-primary/10 text-primary rounded px-2 py-0.5 text-xs font-medium">#{selectedTag}</span>
			<button tabindex={-1} class="text-muted-foreground hover:text-foreground text-xs" onclick={clearTag}>clear</button>
		</div>
	{/if}
	<Command.Input placeholder={searchLabels[searchType].placeholder} bind:value={query} />
	<Command.List>
		{#if (query.trim().length > 0 || selectedTag) && results.length === 0}
			<Command.Empty>{searchLabels[searchType].empty}</Command.Empty>
		{/if}
		{#if results.length > 0}
			<Command.Group heading={selectedTag ? `Documents with #${selectedTag}` : searchLabels[searchType].heading}>
				{#each results as result (result.result_type === 'tag' ? result.tag : `${result.path}${result.anchor ?? ''}`)}
					<Command.Item onSelect={() => selectResult(result)}>
						{#if result.result_type === 'tag'}
							<div class="flex w-full items-center justify-between">
								<div class="flex items-center gap-2">
									<Hash class="text-muted-foreground h-3.5 w-3.5" />
									<span class="text-sm font-medium">{@html highlightMatch(result.tag ?? '', query)}</span>
								</div>
								<span class="text-muted-foreground text-xs">{result.doc_count} docs</span>
							</div>
						{:else if result.result_type === 'heading'}
							<div class="flex items-center gap-2">
								<span class="text-muted-foreground text-[10px] font-bold">H{result.heading_level}</span>
								<div class="flex flex-col gap-0.5">
									<span class="text-sm font-medium">{@html highlightMatch(result.title, query)}</span>
									<span class="text-muted-foreground text-xs">{result.path}</span>
								</div>
							</div>
						{:else if result.result_type === 'file'}
							<div class="flex items-center gap-2">
								<FileText class="text-muted-foreground h-3.5 w-3.5" />
								<div class="flex flex-col gap-0.5">
									<span class="text-sm font-medium">{@html highlightMatch(result.path, query)}</span>
									{#if result.title}
										<span class="text-muted-foreground text-xs">{result.title}</span>
									{/if}
								</div>
							</div>
						{:else}
							<div class="flex flex-col gap-0.5">
								<span class="text-sm font-medium">{result.title || result.path}</span>
								<span class="text-muted-foreground text-xs">{result.path}</span>
								{#if result.snippet}
									<span class="text-muted-foreground mt-0.5 text-xs">
										{@html sanitizeSnippet(result.snippet)}
									</span>
								{/if}
							</div>
						{/if}
					</Command.Item>
				{/each}
			</Command.Group>
		{/if}
	</Command.List>
</Command.Dialog>
