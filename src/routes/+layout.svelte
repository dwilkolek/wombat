<script lang="ts">
	import './styles.css';
	import ErrorBox from '$lib/componets/error-box.svelte';
	import { loading } from '$lib/stores/error-store';
</script>

<div>
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
