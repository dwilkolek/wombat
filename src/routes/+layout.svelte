<script lang="ts">
	import './styles.css';
	import Analytics from '$lib/analytics.svelte';
	import ErrorBox from '$lib/error-box.svelte';
	import { loading } from '$lib/error-store';
</script>

<div>
	<Analytics />
	<ErrorBox />
	{#if $loading}
		<div class="fixed z-50 backdrop-blur-sm w-full h-screen flex items-center">
			<span class="mx-auto my-10">Processing command: {$loading.replace('_', ' ')}</span>
		</div>
	{/if}
	<main class={`px-2`}>
		<slot />
	</main>
</div>
