<script lang="ts">
	import { onMount } from 'svelte';
	import { goto } from '$app/navigation';
	import type { Core, EventObject, Layouts } from 'cytoscape';
	import type { PageData } from './$types';
	import { Settings, X, Shuffle } from '@lucide/svelte';
	import { getGraphStyles } from '$lib/graph-styles';

	let { data }: { data: PageData } = $props();

	let container: HTMLDivElement;
	let cy: Core | null = $state(null);
	let currentLayout: Layouts | null = null;
	let panelOpen = $state(false);

	let edgeLength = $state(80);
	let nodeSpacing = $state(10);
	let nodeSize = $state(8);
	let fontSize = $state(10);
	let showLabels = $state(true);
	let showArrows = $state(true);

	let showTags = $state(true);
	let showAttachments = $state(false);
	let highlightOrphans = $state(true);

	function isDark(): boolean {
		return document.documentElement.classList.contains('dark');
	}

	function smoothFit() {
		if (!cy) return;
		cy.animate({
			fit: { eles: cy.elements(), padding: 40 },
			duration: 400,
			easing: 'ease-out' as never
		});
	}

	function runLayout(randomize: boolean) {
		if (!cy) return;
		if (currentLayout) currentLayout.stop();

		// Place all nodes in a small random cluster before layout
		// so disconnected nodes start close to connected ones
		if (randomize && cy) {
			const cx = cy.width() / 2;
			const cy2 = cy.height() / 2;
			cy.nodes().forEach((node) => {
				node.position({
					x: cx + (Math.random() - 0.5) * 150,
					y: cy2 + (Math.random() - 0.5) * 150
				});
			});
		}

		currentLayout = cy.layout({
			name: 'cola',
			animate: true,
			infinite: false,
			maxSimulationTime: 4000,
			fit: false,
			edgeLength: () => edgeLength,
			nodeSpacing: () => nodeSpacing,
			randomize: false,
			convergenceThreshold: 0.0001,
			ungrabifyWhileSimulating: false,
			centerGraph: true,
			handleDisconnected: true,
			avoidOverlap: true
		// eslint-disable-next-line @typescript-eslint/no-explicit-any
		} as any);
		currentLayout.run();
	}

	function restartLayout() { runLayout(false); }
	function shuffle() { runLayout(true); }

	$effect(() => {
		edgeLength;
		nodeSpacing;
		if (cy) restartLayout();
	});

	$effect(() => {
		if (!cy) return;
		cy.nodes().style({
			width: nodeSize,
			height: nodeSize,
			'font-size': showLabels ? fontSize : 0,
			label: showLabels ? 'data(label)' : ''
		});
	});

	$effect(() => {
		if (!cy) return;
		cy.edges().style({
			'target-arrow-shape': showArrows ? 'triangle' : 'none'
		});
	});

	function updateFilters() {
		if (!cy) return;

		cy.nodes('[node_type = "tag"]').style('display', showTags ? 'element' : 'none');
		cy.nodes('[node_type = "attachment"]').style('display', showAttachments ? 'element' : 'none');
		cy.edges().forEach((edge) => {
			const src = edge.source();
			const tgt = edge.target();
			const srcType = src.data('node_type');
			const tgtType = tgt.data('node_type');
			if (srcType === 'tag' || tgtType === 'tag') {
				edge.style('display', showTags ? 'element' : 'none');
			} else if (srcType === 'attachment' || tgtType === 'attachment') {
				edge.style('display', showAttachments ? 'element' : 'none');
			}
		});

		// Orphan highlighting: doc nodes with zero visible edges
		const dark = isDark();
		cy.nodes('[node_type = "doc"]').forEach((node) => {
			const visibleEdges = node.connectedEdges().filter((e) => e.style('display') !== 'none');
			if (highlightOrphans && visibleEdges.length === 0) {
				node.style({
					'border-width': 2,
					'border-color': '#f59e0b',
					'background-color': '#f59e0b'
				});
			} else {
				node.style({
					'border-width': 0,
					'border-color': '',
					'background-color': dark ? '#a1a1aa' : '#a8a29e'
				});
			}
		});
	}

	$effect(() => {
		showTags;
		showAttachments;
		highlightOrphans;
		updateFilters();
	});

	onMount(() => {
		let destroyed = false;

		(async () => {
			const cytoscape = (await import('cytoscape')).default;
			// @ts-expect-error - no types
			const cola = (await import('cytoscape-cola')).default;
			cytoscape.use(cola);

			if (destroyed) return;

			const dark = isDark();

			const cw = container.clientWidth / 2;
			const ch = container.clientHeight / 2;

			cy = cytoscape({
				container,
				elements: [
					...data.graph.nodes.map((n) => ({
						data: { id: n.id, label: n.label, node_type: n.node_type },
						position: { x: cw + (Math.random() - 0.5) * 150, y: ch + (Math.random() - 0.5) * 150 }
					})),
					...data.graph.edges.map((e) => ({ data: { source: e.source, target: e.target } }))
				],
				style: getGraphStyles(dark),
				layout: { name: 'preset' },
				minZoom: 0.2,
				maxZoom: 4,
				wheelSensitivity: 0.3
			});

			cy.on('mouseover', 'node', (evt: EventObject) => {
				const node = evt.target;
				const neighborhood = node.neighborhood();
				cy!.elements().addClass('dimmed');
				node.removeClass('dimmed').addClass('hover-node');
				neighborhood.nodes().removeClass('dimmed').addClass('hover-neighbor');
				neighborhood.edges().removeClass('dimmed').addClass('hover-edge');
				node.connectedEdges().removeClass('dimmed').addClass('hover-edge');
				container.style.cursor = 'grab';
			});

			cy.on('mouseout', 'node', () => {
				cy!.elements().removeClass('dimmed hover-node hover-neighbor hover-edge');
				container.style.cursor = '';
			});

			cy.on('grab', 'node', () => {
				container.style.cursor = 'grabbing';
			});

			cy.on('free', 'node', () => {
				container.style.cursor = 'grab';
			});

			// Cola's infinite simulation handles the physics:
			// - dragging a node pushes neighbors away naturally
			// - releasing lets the simulation settle with a bounce effect
			// - force diminishes with distance

			cy.on('tap', 'node', (evt: EventObject) => {
				const nodeType = evt.target.data('node_type');
				if (nodeType === 'doc') {
					goto(`/docs/${data.vault}/${evt.target.id()}`);
				}
			});

			// Initial layout + smooth fit after settling
			shuffle();
			updateFilters();
			setTimeout(() => smoothFit(), 600);
		})();

		return () => {
			destroyed = true;
			if (currentLayout) currentLayout.stop();
			cy?.destroy();
			cy = null;
		};
	});
</script>

<svelte:window onkeydown={(e) => { if (e.key === 'Escape') panelOpen = false; }} />

<!-- Click outside panel to close -->
{#if panelOpen}
	<div class="fixed inset-0 z-40" onclick={() => panelOpen = false} role="presentation"></div>
{/if}

<div class="graph-page">
	<div bind:this={container} class="graph-canvas"></div>

	<!-- Floating control -->
	<div class="absolute top-3 right-3 z-50">
		{#if !panelOpen}
			<button
				onclick={() => panelOpen = true}
				class="bg-card border-border text-muted-foreground hover:text-foreground hover:bg-accent flex h-8 w-8 items-center justify-center rounded-lg border shadow-sm transition-colors"
				aria-label="Graph settings"
			>
				<Settings class="h-3.5 w-3.5" />
			</button>
		{:else}
			<div class="bg-card border-border w-60 rounded-lg border p-3 shadow-lg">
				<div class="mb-3 flex items-center justify-between">
					<span class="text-foreground text-xs font-semibold">Settings</span>
					<button
						onclick={() => panelOpen = false}
						class="text-muted-foreground hover:text-foreground text-xs"
						aria-label="Close settings"
					>
						<X class="h-3.5 w-3.5" />
					</button>
				</div>

				<p class="text-muted-foreground mb-1.5 text-[10px] font-medium uppercase tracking-wider">Layout</p>

				<label for="edge-length" class="text-muted-foreground mb-1 block text-[11px]">
					Edge Length <span class="text-foreground font-medium">{edgeLength}</span>
				</label>
				<input id="edge-length" type="range" min="30" max="300" step="10" bind:value={edgeLength}
					class="mb-2 w-full accent-purple-600" />

				<label for="node-spacing" class="text-muted-foreground mb-1 block text-[11px]">
					Node Spacing <span class="text-foreground font-medium">{nodeSpacing}</span>
				</label>
				<input id="node-spacing" type="range" min="5" max="100" step="5" bind:value={nodeSpacing}
					class="mb-3 w-full accent-purple-600" />

				<div class="border-border mb-3 border-t"></div>
				<p class="text-muted-foreground mb-1.5 text-[10px] font-medium uppercase tracking-wider">Appearance</p>

				<label for="node-size" class="text-muted-foreground mb-1 block text-[11px]">
					Node Size <span class="text-foreground font-medium">{nodeSize}px</span>
				</label>
				<input id="node-size" type="range" min="3" max="20" step="1" bind:value={nodeSize}
					class="mb-2 w-full accent-purple-600" />

				<label for="font-size" class="text-muted-foreground mb-1 block text-[11px]">
					Font Size <span class="text-foreground font-medium">{fontSize}px</span>
				</label>
				<input id="font-size" type="range" min="6" max="18" step="1" bind:value={fontSize}
					class="mb-2 w-full accent-purple-600" />

				<div class="mb-3 flex gap-3">
					<label class="text-muted-foreground flex items-center gap-1.5 text-[11px]">
						<input type="checkbox" bind:checked={showLabels} class="accent-purple-600" />
						Labels
					</label>
					<label class="text-muted-foreground flex items-center gap-1.5 text-[11px]">
						<input type="checkbox" bind:checked={showArrows} class="accent-purple-600" />
						Arrows
					</label>
				</div>

				<div class="border-border mb-3 border-t"></div>
				<p class="text-muted-foreground mb-1.5 text-[10px] font-medium uppercase tracking-wider">Filters</p>

				<div class="mb-3 flex flex-col gap-1.5">
					<label class="text-muted-foreground flex items-center gap-1.5 text-[11px]">
						<input type="checkbox" bind:checked={showTags} class="accent-purple-600" />
						<span class="inline-block h-2 w-2 rounded-full" style="background-color: #8b5cf6;"></span>
						Tags
					</label>
					<label class="text-muted-foreground flex items-center gap-1.5 text-[11px]">
						<input type="checkbox" bind:checked={showAttachments} class="accent-purple-600" />
						<span class="inline-block h-2 w-2 rounded-full" style="background-color: #22c55e;"></span>
						Attachments
					</label>
					<label class="text-muted-foreground flex items-center gap-1.5 text-[11px]">
						<input type="checkbox" bind:checked={highlightOrphans} class="accent-purple-600" />
						<span class="inline-block h-2 w-2 rounded-full" style="background-color: #f59e0b;"></span>
						Orphans
					</label>
				</div>

				<div class="border-border mb-3 border-t"></div>

				<div class="flex gap-2">
					<button
						onclick={shuffle}
						class="bg-secondary text-secondary-foreground hover:bg-secondary/80 flex flex-1 items-center justify-center gap-1 rounded-md px-2 py-1 text-xs transition-colors"
					>
						<Shuffle class="h-3 w-3" />
						Shuffle
					</button>
					<button
						onclick={smoothFit}
						class="bg-secondary text-secondary-foreground hover:bg-secondary/80 flex-1 rounded-md px-2 py-1 text-xs transition-colors"
					>
						Fit
					</button>
				</div>
			</div>
		{/if}
	</div>
</div>

<style>
	.graph-page {
		position: relative;
		height: calc(100vh - 49px);
		width: 100%;
		overflow: hidden;
		background-color: var(--background);
	}

	.graph-canvas {
		width: 100%;
		height: 100%;
	}
</style>
