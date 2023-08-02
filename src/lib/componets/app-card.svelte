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
				<h5 class="inline">
					<a href={`/logged/apps/${app}`}>{app}</a>
				</h5>
			</div>
			{#if !details}
				<span class="loading loading-dots loading-lg" />
			{/if}
			{#if details}
				{#each [...details.envs] as [env, value]}
					{#if displayConfig.envs == null || displayConfig.envs.includes(env)}
						<div class="flex flex-row gap-2">
							<div class="font-bold">{env}:</div>
							{#each value.services as service}
								<div class="flex flex-row items-center gap-1">
									<span>{service.version}</span>
									<ServiceCell {service} />
								</div>
							{/each}
							<div>|</div>
							{#each value.dbs as db}
								<div class="flex flex-row items-center gap-1">
									<span>{db.engine_version}</span>
									<DatabaseCell database={db} />
								</div>
							{/each}
						</div>
					{/if}
				{/each}
			{/if}
		</div>
	</div>
{/if}
