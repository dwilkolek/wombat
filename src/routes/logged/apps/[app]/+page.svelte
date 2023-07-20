<script lang="ts">
	import DatabaseCell from '$lib/componets/database-cell.svelte';
	import EnvCard from '$lib/componets/env-card.svelte';
	import ServiceCell from '$lib/componets/service-cell.svelte';
	import StarIcon from '$lib/componets/star-icon.svelte';
	import { serviceDetailStore } from '$lib/stores/service-details-store';
	import { userStore } from '$lib/stores/user-store';
	import type { AppPage } from './+page';

	export let data: AppPage;

	$: detailsStorr = serviceDetailStore(data.app);
	$: details = $detailsStorr;

	$: user = $userStore;
	$: isFavourite = (name: string): boolean => {
		return !!user.tracked_names.find((tracked_name) => tracked_name == name);
	};
</script>

<div class="flex flex-row gap-2 items-center pl-5">
	<button
		class="text-md"
		on:click={() => {
			userStore.favoriteTrackedName(data.app);
		}}
	>
		<StarIcon state={isFavourite(data.app)} />
	</button>
	<h1 class="inline text-xl">
		{data.app}
	</h1>
</div>
{#if !details}
	<span class="loading loading-dots loading-lg" />
{/if}
<div class="flex flex-row flex-wrap">
	{#if details}
		{#each [...details.envs] as [env]}
			<EnvCard app={data.app} {env} />
		{/each}
	{/if}
</div>
