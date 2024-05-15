<script lang="ts">
	import { activeProfilePreferences, userStore } from '$lib/stores/user-store';

	import { allServiceDetailsStore, serviceDetailStore } from '$lib/stores/service-details-store';
	import { wombatProfileStore } from '$lib/stores/available-profiles-store';
	import DatabaseCell from './database-cell.svelte';
	import ServiceCell from './service-cell.svelte';
	import StarIcon from './star-icon.svelte';
	import { ENVIRONMENTS } from '$lib/stores/env-store';
	import type { AwsEnv } from '$lib/types';
	import { taskStore } from '$lib/stores/task-store';
	import DbTaskStatus from './db-task-status.svelte';
	import ServiceTaskStatus from './service-task-status.svelte';
	import AppCardHr from './app-card-hr.svelte';
	import RestartServiceBtn from './restart-service-btn.svelte';
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
		return !!$activeProfilePreferences.tracked_names.find((tracked_name) => tracked_name == name);
	};
</script>

{#if displayConfig.favorite == null || isFavourite(app) === displayConfig.favorite}
	<div class="px-2 py-1 shadow-2xl w-full flex rounded-lg bg-base-300">
		<div class="flex gap-2 flex-col justify-around">
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
					<a class="hover:text-accent underline" href={`/logged/apps/${app}`}>
						{app}
					</a>
				</span>
			</div>
			<!-- <div class="flex gap-2">
				<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4">
					<path stroke-linecap="round" stroke-linejoin="round" d="M9 12.75 11.25 15 15 9.75m-3-7.036A11.959 11.959 0 0 1 3.598 6 11.99 11.99 0 0 0 3 9.749c0 5.592 3.824 10.29 9 11.623 5.176-1.332 9-6.03 9-11.622 0-1.31-.21-2.571-.598-3.751h-.152c-3.196 0-6.1-1.248-8.25-3.285Z" />
				  </svg>
			</div> -->
			{#if details}
				<div class="place-content-end text-xs text-slate-500 font-italic">
					<div class="flex gap-2">
						<span>Synchronized at: {details.timestamp}</span>
						<button
							on:click|preventDefault={() => {
								details && allServiceDetailsStore.refreshOne(details.app);
							}}
						>
							<svg
								xmlns="http://www.w3.org/2000/svg"
								fill="none"
								viewBox="0 0 24 24"
								stroke-width="1.5"
								stroke="currentColor"
								class="w-4 h-4"
							>
								<path
									stroke-linecap="round"
									stroke-linejoin="round"
									d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0 3.181 3.183a8.25 8.25 0 0 0 13.803-3.7M4.031 9.865a8.25 8.25 0 0 1 13.803-3.7l3.181 3.182m0-4.991v4.99"
								/>
							</svg>
						</button>
					</div>
				</div>
			{/if}
		</div>

		{#if !details}
			<span class="loading loading-dots loading-lg" />
		{/if}

		{#if details}
			<div
				class={`grid w-full divide-x divide-base-100`}
				style={`grid-template-columns: repeat(${$wombatProfileStore.environments.length ?? 1}, minmax(0, 1fr));`}
			>
				{#each $wombatProfileStore.environments as enabled_env}
					{@const value = details.envs?.get(enabled_env)}
					{#if displayConfig.envs == null || displayConfig.envs.includes(enabled_env)}
						{@const hasInfraProfile = $wombatProfileStore.infraProfiles.some(
							(infra) => infra.env == enabled_env && infra.app == app
						)}
						<div class={`flex flex-col app-env-cell px-2`}>
							<div class="font-medium text-xs italic flex gap-1 items-center">
								<span>
									<svg
										xmlns="http://www.w3.org/2000/svg"
										viewBox="0 0 16 16"
										fill="currentColor"
										class={`w-3 h-3 ${hasInfraProfile ? 'text-amber-300' : 'opacity-60'}`}
									>
										<path
											fill-rule="evenodd"
											d="M15 8c0 .982-.472 1.854-1.202 2.402a2.995 2.995 0 0 1-.848 2.547 2.995 2.995 0 0 1-2.548.849A2.996 2.996 0 0 1 8 15a2.996 2.996 0 0 1-2.402-1.202 2.995 2.995 0 0 1-2.547-.848 2.995 2.995 0 0 1-.849-2.548A2.996 2.996 0 0 1 1 8c0-.982.472-1.854 1.202-2.402a2.995 2.995 0 0 1 .848-2.547 2.995 2.995 0 0 1 2.548-.849A2.995 2.995 0 0 1 8 1c.982 0 1.854.472 2.402 1.202a2.995 2.995 0 0 1 2.547.848c.695.695.978 1.645.849 2.548A2.996 2.996 0 0 1 15 8Zm-3.291-2.843a.75.75 0 0 1 .135 1.052l-4.25 5.5a.75.75 0 0 1-1.151.043l-2.25-2.5a.75.75 0 1 1 1.114-1.004l1.65 1.832 3.7-4.789a.75.75 0 0 1 1.052-.134Z"
											clip-rule="evenodd"
										/>
									</svg></span
								><span>{enabled_env}:</span>
							</div>
							<div class="flex gap-1 app-env-cell-stack">
								{#if value}
									{#each value.services.filter((service) => !service.error) as service}
										{@const task = tasks.find((task) => task.arn == service.arn)}

										<div class="flex flex-row items-center gap-1 px-1">
											<ServiceCell {service} />
											<div class="flex gap-2 justify-between items-center grow">
												<span class="truncate">{service.version}</span>

												<RestartServiceBtn {service} />
												<AppCardHr {task} />
												<ServiceTaskStatus {task} {service} />
											</div>
										</div>
									{/each}
									{#each value.services.filter((service) => service.error) as service}
										{@const task = tasks.find((task) => task.arn == service.arn)}
										<div class="flex flex-row items-center gap-1 px-1 text-rose-800 text-sm">
											{service.error}
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
	.app-env-cell-stack {
		flex-direction: column;
	}
</style>
