<script lang="ts">
	import '../styles.css';
	import ErrorBox from '$lib/componets/error-box.svelte';
	import { loading } from '$lib/stores/error-store';
	import { invoke } from '@tauri-apps/api/core';
	interface Props {
		children?: import('svelte').Snippet;
	}

	let { children }: Props = $props();
</script>

<div>
	<ErrorBox />
	{#await invoke('is_debug') then isDebug}
		{#if !isDebug}
			<script
				defer
				src="https://umami.wilkolek.eu/script.js"
				data-website-id="dc4bbfa3-79fe-4f04-bd34-92a55956847e"
				data-host-url="https://umami.wilkolek.eu"
			></script>
		{/if}
	{/await}

	{#if $loading}
		<div class="fixed z-50 backdrop-blur-sm w-full h-screen flex items-center">
			<span class="mx-auto my-10">Processing command: {$loading.replace('_', ' ')}</span>
		</div>
	{/if}
	<main>
		{@render children?.()}
	</main>
</div>
