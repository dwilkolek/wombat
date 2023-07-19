<script lang="ts">
	import { userStore } from '$lib/user-store';
	import AppCard from './app-card.svelte';
	const user = $userStore;
</script>

<svelte:head>
	<title>APPS</title>
	<meta name="description" content="Wombat" />
</svelte:head>
<div class="flex flex-wrap gap-2">
	{#each user.tracked_names as app}
		<AppCard {app} />
	{/each}
</div>
