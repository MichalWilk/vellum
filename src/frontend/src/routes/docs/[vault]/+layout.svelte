<script lang="ts">
	import type { LayoutData } from './$types';
	import type { Snippet } from 'svelte';
	import type { FileNode } from '$lib/types';
	import { page } from '$app/state';
	import { ScrollArea } from '$lib/components/ui/scroll-area/index.js';
	import { Separator } from '$lib/components/ui/separator/index.js';
	import { FolderTree, ChevronRight, ChevronDown, FileText, Folder, FolderOpen } from '@lucide/svelte';

	let { data, children }: { data: LayoutData; children: Snippet } = $props();

	let collapsed: Record<string, boolean> = $state({});

	function toggle(path: string) {
		collapsed[path] = !collapsed[path];
	}

	function isActive(nodePath: string): boolean {
		return page.url.pathname === `/docs/${data.vault}/${nodePath}`;
	}
</script>

{#snippet fileTree(nodes: FileNode[], depth: number)}
	{#each nodes as node}
		{#if node.type === 'dir'}
			<div>
				<button
					class="text-muted-foreground hover:bg-accent hover:text-accent-foreground flex w-full items-center gap-1 rounded-md px-2 py-1.5 text-left text-sm transition-colors"
					style="padding-left: {depth * 12 + 8}px"
					onclick={() => toggle(node.path)}
				>
					{#if collapsed[node.path]}
						<ChevronRight class="h-3.5 w-3.5 shrink-0 opacity-60" />
						<Folder class="h-3.5 w-3.5 shrink-0" />
					{:else}
						<ChevronDown class="h-3.5 w-3.5 shrink-0 opacity-60" />
						<FolderOpen class="h-3.5 w-3.5 shrink-0" />
					{/if}
					<span class="font-medium">{node.name}</span>
				</button>
				{#if !collapsed[node.path] && node.children}
					{@render fileTree(node.children, depth + 1)}
				{/if}
			</div>
		{:else}
			<a
				href="/docs/{data.vault}/{node.path}"
				class="flex items-center gap-1 truncate rounded-md px-2 py-1.5 text-sm transition-colors {isActive(node.path)
					? 'bg-accent text-accent-foreground font-medium'
					: 'text-muted-foreground hover:bg-accent/50 hover:text-accent-foreground'}"
				style="padding-left: {depth * 12 + 24}px"
			>
				<FileText class="h-3.5 w-3.5 shrink-0" />
				{node.name.replace(/\.md$/, '')}
			</a>
		{/if}
	{/each}
{/snippet}

<div>
	<aside class="bg-muted/40 fixed top-[49px] left-0 h-[calc(100vh-49px)] w-70 border-r">
		<div class="p-3 pb-2">
			<h2 class="text-muted-foreground mb-1 flex items-center gap-1.5 px-2 text-xs font-semibold tracking-wide uppercase">
				<FolderTree class="h-3.5 w-3.5" />
				Files
			</h2>
		</div>
		<Separator />
		<ScrollArea class="h-[calc(100vh-49px-44px)] px-3 py-2">
			{@render fileTree(data.tree, 0)}
		</ScrollArea>
	</aside>
	<main class="ml-70 min-h-[calc(100vh-49px)] min-w-0">
		{@render children()}
	</main>
</div>
