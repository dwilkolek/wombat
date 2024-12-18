<script lang="ts">
	import { activeProfilePreferences, userStore } from '$lib/stores/user-store';

	import { allServiceDetailsStore, serviceDetailStore } from '$lib/stores/service-details-store';
	import { wombatProfileStore } from '$lib/stores/available-profiles-store';
	import DatabaseCell from './database-cell.svelte';
	import ServiceCell from './service-cell.svelte';
	import StarIcon from './star-icon.svelte';
	import type { AwsEnv } from '$lib/types';
	import { taskStore } from '$lib/stores/task-store';
	import DbTaskStatus from './db-task-status.svelte';
	import ServiceTaskStatus from './service-task-status.svelte';
	import AppCardHr from './app-card-hr.svelte';
	import { format } from 'date-fns/format';
	import DeployOrRestartServiceBtn from './deploy-or-restart-service-btn.svelte';
	import RemoveNonPlatformTaskDefinitionsBtn from './remove-non-platform-task-definitions-btn.svelte';
	interface Props {
		app: string;
		displayConfig: {
			envs: AwsEnv[] | null;
			favorite: boolean | null;
		};
	}

	let { app, displayConfig }: Props = $props();

	let details = serviceDetailStore(app);

	let isFavourite = $derived((name: string): boolean => {
		return !!$activeProfilePreferences.tracked_names.find((tracked_name) => tracked_name == name);
	});
</script>

{#if displayConfig.favorite == null || isFavourite(app) === displayConfig.favorite}
	<div class="px-2 py-1 shadow-2xl w-full flex rounded-lg bg-base-300">
		<div class="flex gap-2 flex-col justify-around">
			<div class="min-w-80 w-80 flex flex-row gap-2 items-center text-md">
				<button
					class="text-xs"
					data-umami-event="favorite_app_toggle"
					data-umami-event-uid={$userStore.id}
					onclick={() => {
						userStore.favoriteTrackedName(app);
					}}
				>
					<StarIcon isSelected={isFavourite(app)} />
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
			{#if $details}
				<div class="place-content-end text-xs text-slate-500 font-italic">
					<div class="flex gap-2">
						<span>Synchronized at: {format($details.timestamp, 'yyyy-MM-dd HH:mm:ss')}</span>
						<button
							aria-label="Refresh"
							data-umami-event="app_refresh"
							data-umami-event-uid={$userStore.id}
							onclick={(e) => {
								e.preventDefault();
								allServiceDetailsStore.refreshOne($details.app);
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

		{#if !$details}
			<span class="loading loading-dots loading-lg"></span>
		{/if}

		{#if $details}
			<div
				class={`grid w-full divide-x divide-base-100`}
				style={`grid-template-columns: repeat(${$wombatProfileStore.environments.length ?? 1}, minmax(0, 1fr));`}
			>
				{#each $wombatProfileStore.environments as enabled_env}
					{@const value = $details.envs?.get(enabled_env)}
					{#if displayConfig.envs == null || displayConfig.envs.includes(enabled_env)}
						{@const hasInfraProfile = $wombatProfileStore.infraProfiles.some(
							(infra) => infra.env == enabled_env && infra.app == app
						)}
						<div class={`flex flex-col app-env-cell px-2`}>
							<div class="font-medium text-xs italic flex gap-1 items-center">
								<span class={`${hasInfraProfile ? '' : 'opacity-60'}`}>{enabled_env}: </span>
							</div>
							<div class="flex gap-1 app-env-cell-stack">
								{#if value}
									{#each value.services.filter((service) => !service.error) as service}
										{@const task = $taskStore.find((task) => task.arn == service.arn)}

										<div class="flex flex-row items-center gap-1 px-1">
											<ServiceCell {service} />
											<div class="flex gap-2 justify-between items-center grow">
												<div
													class="flex flex-col tooltip tooltip-left"
													data-tip={`Deployed ${
														service.version.length < 18
															? service.version
															: service.version.substring(0, 15) + '...'
													} at: ${
														service.task_registered_at
															? format(service.task_registered_at, 'yyyy-MM-dd HH:mm:ss')
															: ''
													}`}
												>
													{service.version.length < 18
														? service.version
														: service.version.substring(0, 15) + '...'}
												</div>
												<DeployOrRestartServiceBtn {service} />
												<RemoveNonPlatformTaskDefinitionsBtn {service} />
												<AppCardHr {task} />
												<ServiceTaskStatus {task} {service} />
											</div>
										</div>
									{/each}
									{#each value.services.filter((service) => service.error) as service}
										<div class="flex flex-row items-center gap-1 px-1 text-rose-800 text-sm">
											{service.error}
										</div>
									{/each}

									{#each value.dbs as db}
										{@const task = $taskStore.find((task) => task.arn == db.arn)}
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
