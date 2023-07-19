<script lang="ts">
	import { execute } from '$lib/error-store';
	import { taskStore } from '$lib/task-store';
	import { AwsEnv, type DbInstance } from '$lib/types';
	import { userStore } from '$lib/user-store';
	import DbSecretBtn from '$lib/db-secret-btn.svelte';
	import { ask } from '@tauri-apps/api/dialog';

	import { serviceDetailStore } from '$lib/service-details-store';
	import DatabaseCell from './database-cell.svelte';
	import ServiceCell from './service-cell.svelte';
	import StarIcon from '$lib/star-icon.svelte';
	export let app: string;

	$: detailsStorr = serviceDetailStore(app);
	$: details = $detailsStorr;

	$: user = $userStore;
	$: isFavourite = (name: string): boolean => {
		return !!user.tracked_names.find((tracked_name) => tracked_name == name);
	};
</script>

<div class="card card-compact w-96 bg-base-100 shadow-xl">
	<div class="card-body">
		<div class="card-title flex flex-row gap-2 items-center text-md">
			<button
				class="text-xs"
				on:click={() => {
					userStore.favoriteTrackedName(app);
				}}
			>
				<StarIcon state={isFavourite(app)} />
			</button>
			<h5 class="inline">{app}</h5>
		</div>
		{#if !details}
			<span class="loading loading-dots loading-lg" />
		{/if}
		{#if details}
			{#each [...details.envs] as [env, value]}
				<div class="flex flex-row gap-2">
					<div class="font-bold">{env}:</div>
					{#each value.services as service}
						<ServiceCell {service} />
					{/each}
					{#each value.dbs as db}
						<DatabaseCell database={db} />
					{/each}
				</div>
			{/each}
		{/if}
	</div>
</div>
