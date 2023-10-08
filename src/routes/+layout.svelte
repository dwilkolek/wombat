<script lang="ts">
	import './styles.css';
	import Analytics from '$lib/componets/analytics.svelte';
	import ErrorBox from '$lib/componets/error-box.svelte';
	import { loading } from '$lib/stores/error-store';
</script>

<div>
	<Analytics />
	<ErrorBox />
	{#if $loading}
		<div class="fixed z-50 backdrop-blur-sm w-full h-screen flex items-center">
			<span class="mx-auto my-10">Processing command: {$loading.replace('_', ' ')}</span>
		</div>
	{/if}
	<main>
		<slot />
	</main>
</div>
