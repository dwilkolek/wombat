<script lang="ts">
	import { userStore } from '$lib/stores/user-store';

	import { serviceDetailStore } from '$lib/stores/service-details-store';
	import DatabaseCell from './database-cell.svelte';
	import ServiceCell from './service-cell.svelte';
	import StarIcon from './star-icon.svelte';
	import { ENVIRONMENTS } from '$lib/stores/env-store';
	import type { AwsEnv } from '$lib/types';
	import { taskStore } from '$lib/stores/task-store';
	import DbTaskStatus from './db-task-status.svelte';
	import ServiceTaskStatus from './service-task-status.svelte';
	import AppCardHr from './app-card-hr.svelte';
	export let app: string;
	export let displayConfig: {
		envs: AwsEnv[] | null;
		favorite: boolean | null;
	};

	$: detailsStorr = serviceDetailStore(app);
	$: details = $detailsStorr;

	$: tasks = $taskStore;

	$: user = $userStore;
	$: isFavourite = (name: string): boolean => {
		return !!user.tracked_names.find((tracked_name) => tracked_name == name);
	};
</script>

{#if displayConfig.favorite == null || isFavourite(app) === displayConfig.favorite}
	<div class="px-2 py-1 shadow-2xl w-full flex rounded-lg bg-base-300">
		<div class="min-w-80 w-80 flex flex-row gap-2 items-center text-md">
			<button
				class="text-xs"
				on:click={() => {
					userStore.favoriteTrackedName(app);
				}}
			>
				<StarIcon state={isFavourite(app)} />
			</button>
			<span class="inline text-base">
				<a class="hover:text-accent underline" href={`/logged/apps/${app}`}>{app}</a>
			</span>
		</div>
		{#if !details}
			<span class="loading loading-dots loading-lg" />
		{/if}
		{#if details}
			<div
				class={`grid w-full divide-x divide-base-100`}
				style={`grid-template-columns: repeat(${displayConfig.envs?.length ?? 1}, minmax(0, 1fr));`}
			>
				{#each ENVIRONMENTS as enabled_env}
					{@const value = details.envs?.get(enabled_env)}
					{#if displayConfig.envs == null || displayConfig.envs.includes(enabled_env)}
						<div class={`flex flex-col app-env-cell px-2`}>
							<div class="font-medium w-16 text-xs italic">{enabled_env}:</div>
							<div class="flex gap-1 app-env-cell-stack">
								{#if value}
									{#each value.services as service}
										{@const task = tasks.find((task) => task.arn == service.arn)}
										<div class="flex flex-row items-center gap-1 px-1">
											<ServiceCell {service} />
											<div class="flex gap-2 justify-between items-center grow">
												<span class="truncate">{service.version}</span>
												<AppCardHr {task} />
												<ServiceTaskStatus {task} {service} />
											</div>
										</div>
									{/each}

									{#each value.dbs as db}
										{@const task = tasks.find((task) => task.arn == db.arn)}
										<div class="flex flex-row items-center gap-1 px-1">
											<DatabaseCell database={db} />
											<div class="flex gap-2 justify-between items-center grow">
												<span class="truncate">{db.engine_version}</span>
												<AppCardHr {task} />
												<DbTaskStatus {task} {db} />
											</div>
										</div>
									{/each}
								{/if}
							</div>
						</div>
					{/if}
				{/each}
			</div>
		{/if}
	</div>
{/if}

<style>
	/* .app-env-cell {
		container: envcell / inline-size;
	} */

	/* @container envcell (width < 300px) { */
	.app-env-cell-stack {
		flex-direction: column;
	}
	/* } */
</style>
