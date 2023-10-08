<script lang="ts">
	import { userStore } from '$lib/stores/user-store';

	import { serviceDetailStore } from '$lib/stores/service-details-store';
	import DatabaseCell from './database-cell.svelte';
	import ServiceCell from './service-cell.svelte';
	import StarIcon from './star-icon.svelte';
	import type { AwsEnv } from '$lib/types';
	export let app: string;
	export let displayConfig: {
		envs: AwsEnv[] | null;
		favorite: boolean | null;
	};

	$: detailsStorr = serviceDetailStore(app);
	$: details = $detailsStorr;

	$: user = $userStore;
	$: isFavourite = (name: string): boolean => {
		return !!user.tracked_names.find((tracked_name) => tracked_name == name);
	};
</script>

{#if displayConfig.favorite == null || isFavourite(app) === displayConfig.favorite}
	<div class="px-2 py-1 bg-base-300 shadow-2xl w-full gap-1 flex rounded-2xl">
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
			<div class="flex gap-2 flex-wrap w-full">
				{#each [...details.envs] as [env, value]}
					{#if displayConfig.envs == null || displayConfig.envs.includes(env)}
						<div class="flex flex-col w-64">
							<div class="font-bold w-16 text-xs italic">{env}:</div>
							<div class="flex gap-3 grow">
								{#each value.services as service}
									<div class="flex flex-row items-center gap-1 px-1">
										<ServiceCell {service} />
										<span class="truncate w-20">{service.version}</span>
									</div>
								{/each}

								{#each value.dbs as db}
									<div class="flex flex-row items-center gap-1 px-1">
										<DatabaseCell database={db} />
										<span class="truncate w-20">{db.engine_version}</span>
									</div>
								{/each}
							</div>
						</div>
					{/if}
				{/each}
			</div>
		{/if}
	</div>
{/if}
