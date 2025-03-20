<script lang="ts">
	import AppEnvCard from '$lib/componets/app-env-card.svelte';
	import StarIcon from '$lib/componets/star-icon.svelte';
	import { serviceDetailStore } from '$lib/stores/service-details-store';
	import { activeProfilePreferences, userStore } from '$lib/stores/user-store';
	import type { AppPage } from './+page';

	interface Props {
		data: AppPage;
	}

	let { data }: Props = $props();

	let detailsStorr = $derived(serviceDetailStore(data.app));
	let details = $derived($detailsStorr);

	let isFavourite = $derived((name: string): boolean => {
		return !!$activeProfilePreferences.tracked_names.find((tracked_name) => tracked_name == name);
	});
</script>

<svelte:head>
	<title>APP {data.app}</title>
	<meta name="description" content="Wombat" />
</svelte:head>

<div class="flex flex-row gap-2 items-center pl-5">
	<button
		class="text-md"
		data-umami-event="favorite_app_toggle"
		data-umami-event-uid={$userStore.id}
		onclick={() => {
			userStore.favoriteTrackedName(data.app);
		}}
	>
		<StarIcon isSelected={isFavourite(data.app)} />
	</button>
	<h1 class="inline text-xl">
		{data.app}
	</h1>
</div>
{#if !details}
	<span class="loading loading-dots loading-lg"></span>
{/if}
<div class="flex flex-col">
	{#if details}
		{#each [...details.envs] as [env] (env)}
			<AppEnvCard app={data.app} {env} />
		{/each}
	{/if}
</div>
