<script lang="ts">
	import PikachuIcon from '$lib/images/pikachu.png';
	import PsyduckIcon from '$lib/images/psyduck.png';
	import PokeballIcon from '$lib/images/pokeball.png';
	import { execute } from '$lib/stores/error-store';
	import { featuresStore } from '$lib/stores/feature-store';
	import { userStore } from '$lib/stores/user-store';
	import { availableProfilesStore } from '$lib/stores/available-profiles-store';

	let userConfig = $derived($userStore);
	let loadingAwsConfigs = $state(false);

	let isLoading = $derived(loadingAwsConfigs || $featuresStore.loading);
</script>

{#if isLoading}
	<button disabled={true} class="btn">
		<img class="h-6" alt="In progress" src={PokeballIcon} />
		{userConfig.last_used_profile}
	</button>
{:else}
	<button
		class="btn"
		onclick={async () => {
			try {
				loadingAwsConfigs = true;
				await execute('reload_aws_config', undefined, true);
				await availableProfilesStore.refresh();
				await featuresStore.refreshFeatures();
			} finally {
				loadingAwsConfigs = false;
			}
		}}
		data-umami-event="fs_refresh"
		data-umami-event-uid={$userStore.id}
	>
		{#if $featuresStore.devWay}
			<img class="h-6" alt="dev-way" src={PikachuIcon} />
		{:else}
			<img class="h-6" alt="platform-way" src={PsyduckIcon} />
		{/if}
		{userConfig.last_used_profile}
	</button>
{/if}
