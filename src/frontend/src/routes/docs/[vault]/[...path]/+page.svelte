<script lang="ts">
	import { onMount } from 'svelte';
	import hljs from 'highlight.js';
	import type { PageData } from './$types';
	import { Badge } from '$lib/components/ui/badge/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { Tag, Calendar, User } from '@lucide/svelte';
	import { openSearchWithQuery } from '$lib/search';

	let { data }: { data: PageData } = $props();
	let contentEl: HTMLElement | undefined = $state();

	function highlightCode() {
		if (!contentEl) return;
		const blocks = contentEl.querySelectorAll('pre code');
		for (const block of blocks) {
			hljs.highlightElement(block as HTMLElement);
		}
	}

	function handleTagClick(tag: string) {
		openSearchWithQuery(`#${tag}`);
	}

	function onContentClick(e: MouseEvent) {
		const target = e.target as HTMLElement;
		if (target.classList.contains('vellum-tag')) {
			e.preventDefault();
			const tagName = target.textContent?.replace(/^#/, '') ?? '';
			if (tagName) handleTagClick(tagName);
		}
	}

	onMount(() => {
		loadHljsTheme();
		highlightCode();

		const observer = new MutationObserver(() => loadHljsTheme());
		observer.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] });
		return () => observer.disconnect();
	});

	$effect(() => {
		data.doc;
		highlightCode();
	});

	function loadHljsTheme() {
		const dark = document.documentElement.classList.contains('dark');
		const id = 'hljs-theme';
		let link = document.getElementById(id) as HTMLLinkElement | null;
		const href = dark
			? 'https://cdn.jsdelivr.net/gh/highlightjs/cdn-release@11/build/styles/github-dark.min.css'
			: 'https://cdn.jsdelivr.net/gh/highlightjs/cdn-release@11/build/styles/github.min.css';

		if (link) {
			link.href = href;
		} else {
			link = document.createElement('link');
			link.id = id;
			link.rel = 'stylesheet';
			link.href = href;
			document.head.appendChild(link);
		}
	}
</script>

<article class="mx-auto max-w-3xl px-8 py-8">
	{#if data.doc.frontmatter.title}
		<h1 class="text-foreground mb-2 text-3xl font-bold">
			{data.doc.frontmatter.title}
		</h1>
	{/if}

	{#if Array.isArray(data.doc.frontmatter.tags)}
		<div class="mb-6 flex flex-wrap gap-2">
			{#each data.doc.frontmatter.tags as tag}
				<button type="button" onclick={() => handleTagClick(String(tag))} class="cursor-pointer">
					<Badge variant="secondary" class="gap-1 transition-colors hover:bg-primary/10">
						<Tag class="h-3 w-3" />
						{tag}
					</Badge>
				</button>
			{/each}
		</div>
	{/if}

	<!-- svelte-ignore a11y_click_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="prose prose-zinc dark:prose-invert max-w-none" bind:this={contentEl} onclick={onContentClick}>
		{@html data.doc.content}
	</div>

	{#if data.doc.last_modified}
		<Separator class="mt-8" />
		<footer class="text-muted-foreground flex items-center gap-4 pt-4 text-sm">
			<span class="flex items-center gap-1">
				<Calendar class="h-3.5 w-3.5" />
				{new Date(data.doc.last_modified).toLocaleDateString()}
			</span>
			{#if data.doc.last_modified_by}
				<span class="flex items-center gap-1">
					<User class="h-3.5 w-3.5" />
					{data.doc.last_modified_by}
				</span>
			{/if}
		</footer>
	{/if}
</article>
