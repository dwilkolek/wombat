<script lang="ts">
	import { activeProfilePreferences, userStore } from '$lib/stores/user-store';
	import { AwsEnv } from '$lib/types';
	import { invoke } from '@tauri-apps/api/core';
	import AppCard from '$lib/componets/app-card.svelte';
	import { wombatProfileStore } from '$lib/stores/available-profiles-store';

	let selectedClusters = $derived($activeProfilePreferences.preffered_environments);

	let columnToggleHandler = $derived((env: AwsEnv, e: { currentTarget: { checked: boolean } }) => {
		if (!e.currentTarget.checked) {
			userStore.savePrefferedEnvs([
				...selectedClusters.filter((selectedEnv) => env != selectedEnv)
			]);
		} else {
			userStore.savePrefferedEnvs([...selectedClusters, env]);
		}
	});

	let envs = $derived($wombatProfileStore.environments);
	let discoverValue: string = $state('');
	let discovered: Promise<string[]> | undefined = $state(undefined);
</script>

<svelte:head>
	<title>APPS</title>
	<meta name="description" content="Wombat" />
</svelte:head>
<div class="bg-base-100 flex flex-row justify-between px-2 sticky top-[68px] z-50">
	<form
		class="flex flex-row gap-2 mb-2"
		onsubmit={async (e) => {
			e.preventDefault();
			discovered = invoke('discover', { name: discoverValue });
		}}
	>
		<input
			type="text"
			autocomplete="off"
			autocorrect="off"
			autocapitalize="off"
			spellcheck="false"
			placeholder="Discover by name"
			bind:value={discoverValue}
			class="input input-bordered w-full max-w-xs input-sm"
		/>
		<button
			class="btn btn-primary btn-sm"
			type="submit"
			data-umami-event="app_search_start"
			data-umami-event-uid={$userStore.id}
		>
			Discover
		</button>
		{#if discovered}
			<button
				class="btn btn-secondary btn-sm"
				type="button"
				data-umami-event="app_search_reset"
				data-umami-event-uid={$userStore.id}
				onclick={() => {
					discoverValue = '';
					discovered = undefined;
				}}
			>
				Reset
			</button>
		{/if}
	</form>
	<div class="flex flex-row flex-wrap gap-5">
		{#each envs as env (env)}
			<div class="form-control">
				<label class="cursor-pointer label flex flex-row gap-2">
					<input
						type="checkbox"
						class="toggle toggle-accent"
						data-umami-event="selected_env_toggle"
						data-umami-event-uid={$userStore.id}
						checked={selectedClusters.includes(env)}
						onchange={(e) => {
							columnToggleHandler(env, e);
						}}
					/>
					<span class="label-text">{env}</span>
				</label>
			</div>
		{/each}
	</div>
</div>
<div class="flex flex-col gap-2 grow mx-2">
	<div class="flex flex-wrap gap-2">
		{#if discovered}
			{#await discovered}
				<span class="loading loading-dots loading-lg"></span>
			{:then discoverValue}
				{#each discoverValue as discoveredApp}
					<AppCard
						app={discoveredApp}
						displayConfig={{
							envs: selectedClusters,
							favorite: false
						}}
					/>
				{/each}
			{/await}
		{/if}
	</div>
	<div class="flex flex-wrap gap-2">
		{#each $activeProfilePreferences.tracked_names as app}
			<AppCard
				{app}
				displayConfig={{
					envs: selectedClusters,
					favorite: true
				}}
			/>
		{/each}
	</div>
</div>
