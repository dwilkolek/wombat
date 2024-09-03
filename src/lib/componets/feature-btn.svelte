<script lang="ts">
	import PikachuIcon from '$lib/images/pikachu.png';
	import PsyduckIcon from '$lib/images/psyduck.png';
	import PokeballIcon from '$lib/images/pokeball.png';
	import { featuresStore } from '$lib/stores/feature-store';
	import { userStore } from '$lib/stores/user-store';
</script>

{#if $featuresStore.loading}
	<button disabled={true}>
		<img class="h-6" alt="In progress" src={PokeballIcon} />
	</button>
{:else}
	<button
		on:click={featuresStore.refreshFeatures}
		data-umami-event="fs_refresh"
		data-umami-event-uid={$userStore.id}
	>
		{#if $featuresStore.devWay}
			<img class="h-6" alt="dev-way" src={PikachuIcon} />
		{:else}
			<img class="h-6" alt="platform-way" src={PsyduckIcon} />
		{/if}
	</button>
{/if}
