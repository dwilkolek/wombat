<script lang="ts">
	import './styles.css';
	import Analytics from '$lib/analytics.svelte';
</script>

<div>
	<Analytics />

	<main class="px-2">
		<slot />
	</main>
</div>
