<script lang="ts">
	import { activeProfilePreferences, userStore } from '$lib/stores/user-store';
	import { AwsEnv } from '$lib/types';
	import { invoke } from '@tauri-apps/api';
	import AppCard from '$lib/componets/app-card.svelte';
	import { wombatProfileStore } from '$lib/stores/available-profiles-store';

	$: selectedClusters = $activeProfilePreferences.preffered_environments;

	$: columnToggleHandler = (env: AwsEnv, e: { currentTarget: { checked: boolean } }) => {
		if (!e.currentTarget.checked) {
			userStore.savePrefferedEnvs([
				...selectedClusters.filter((selectedEnv) => env != selectedEnv)
			]);
		} else {
			userStore.savePrefferedEnvs([...selectedClusters, env]);
		}
	};

	$: envs = $wombatProfileStore.environments;
	let discoverValue: string = '';
	let discovered: Promise<string[]> | undefined = undefined;
</script>

<svelte:head>
	<title>APPS</title>
	<meta name="description" content="Wombat" />
</svelte:head>
<div class="bg-base-100 flex flex-row justify-between px-2 sticky top-[68px] z-50">
	<form
		class="flex flex-row gap-1 mb-2"
		on:submit|preventDefault={async () => {
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
			class="input input-bordered w-full max-w-xs"
		/>
		<button class="btn btn-primary" type="submit"> Discover </button>
		{#if discovered}
			<button
				class="btn btn-secondary"
				type="button"
				on:click={() => {
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
						checked={selectedClusters.includes(env)}
						on:change={(e) => {
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
				<span class="loading loading-dots loading-lg" />
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
