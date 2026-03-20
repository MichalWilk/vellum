<script lang="ts">
	import type { PageData } from './$types';
	import * as Card from '$lib/components/ui/card/index.js';
	import { Button } from '$lib/components/ui/button/index.js';
	import { Library, StickyNote, BookOpen, Network } from '@lucide/svelte';

	let { data }: { data: PageData } = $props();

	function vaultIcon(description?: string | null) {
		if (!description) return Library;
		const lower = description.toLowerCase();
		if (lower.includes('notes')) return StickyNote;
		if (lower.includes('wiki')) return BookOpen;
		return Library;
	}
</script>

<div class="mx-auto max-w-3xl px-8 py-12">
	<h1 class="text-foreground mb-8 text-3xl font-bold">Vaults</h1>
	<div class="grid gap-4 sm:grid-cols-2">
		{#each data.vaults as vault (vault.name)}
			{@const Icon = vaultIcon(vault.description)}
			<Card.Root>
				<Card.Header>
					<Card.Title class="flex items-center gap-2">
						<Icon class="h-4 w-4" />
						{vault.name}
					</Card.Title>
					{#if vault.description}
						<Card.Description>{vault.description}</Card.Description>
					{/if}
				</Card.Header>
				<Card.Footer>
					<div class="flex gap-2">
						<Button size="sm" href="/docs/{vault.name}">Browse</Button>
						<Button size="sm" variant="outline" href="/graph/{vault.name}" class="gap-1">
							<Network class="h-3.5 w-3.5" />
							Graph
						</Button>
					</div>
				</Card.Footer>
			</Card.Root>
		{/each}
	</div>
</div>
