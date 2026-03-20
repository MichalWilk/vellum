<script lang="ts">
	import { page } from '$app/state';
	import { Button } from '$lib/components/ui/button/index.js';
	import { AlertCircle } from '@lucide/svelte';
</script>

<div class="flex min-h-[calc(100vh-49px)] items-center justify-center">
	<div class="text-center">
		<AlertCircle class="text-muted-foreground mx-auto mb-4 h-12 w-12" />
		<p class="text-muted-foreground text-7xl font-bold">{page.status}</p>
		<p class="text-muted-foreground mt-2 text-lg">{page.error?.message || 'Something went wrong'}</p>
		<div class="mt-6 flex justify-center gap-3">
			<Button variant="outline" href="/docs">Back to docs</Button>
		</div>
	</div>
</div>
