<script lang="ts">
	import Icon from 'svelte-icon/Icon.svelte';
	import star from '$lib/images/star-solid.svg?raw';
	import { AwsEnv } from '$lib/types';
	import { homeStore } from '$lib/home-store';
	import { discoverStore } from '$lib/discover-store';
	import DatabaseCell from './database-cell.svelte';
	import ServiceCell from './service-cell.svelte';
	import { userStore } from '$lib/user-store';
	import { envStore } from '$lib/env-store';
	import { clusterStore } from '$lib/cluster-store';
	import StarIcon from '$lib/star-icon.svelte';
	$: homeStore.init();
	$: homeEntries = $homeStore ? $homeStore.sort((a, b) => a.tracked_name.localeCompare(b.tracked_name)) : [];
	$: clusters = clusterStore.clusters
	$: usedEnvs = homeEntries.flatMap(e => [...e.dbs.map(db => db.env), ...Object.values(e.services).map(s => s.env)])
	$: clustersFiltered = $clusters.filter(cluster =>usedEnvs.includes(cluster.env));

	let discoverValue: string = '';
</script>

<svelte:head>
	<title>HOME</title>
	<meta name="description" content="Wombat" />
</svelte:head>
<div class="h-full block">
	<div class="my-4 p-2 pb-5">
		<form
			class="flex flex-row gap-1 mb-2"
			on:submit|preventDefault={async () => {
				discoverStore.discover(discoverValue);
			}}
		>
			<input
				type="text"
				autocomplete="false"
				autocorrect="off"
				autocapitalize="off"
				spellcheck="false"
				placeholder="Discover by name"
				bind:value={discoverValue}
				class="input input-bordered w-full max-w-xs"
			/>
			<button class="btn btn-primary" type="submit"> Discover </button>
			{#if $discoverStore}<button
					class="btn btn-secondary"
					type="button"
					on:click={() => {
						discoverValue = '';
						$discoverStore = undefined;
					}}
				>
					Reset
				</button>
			{/if}
		</form>
		{#if $discoverStore}
			<table class="table w-full table-zebra table-compact">
				<thead class="sticky top-0">
					<tr>
						<th>
							<div class="flex gap-2">APP</div>
						</th>						
						{#each clustersFiltered as cluster}
							<th class="w-40">{cluster.arn.split("/")[1]}</th>
						{/each}
						<th class="w-40">{AwsEnv.DEV}</th>
						<th class="w-40">{AwsEnv.DEMO}</th>
						<th class="w-40">{AwsEnv.PROD}</th>
						<th class="w-10" />
					</tr>
				</thead>
				<tbody class="overflow-y-auto max-h-96">
					{#if $discoverStore.length == 0}
						<tr>
							<td colspan="4">Nothing new was found</td>
						</tr>
					{/if}
					{#each $discoverStore as entry}
					<tr>
						<td>
							<span class="font-bold flex flex-row align-middle gap-1">
								<button
										on:click={() => {
											userStore.favoriteTrackedName(entry.tracked_name);
										}}
									>
										<StarIcon state={false} />
								</button>
								{entry.tracked_name}
							</span>
						</td>
											
						{#each clustersFiltered as cluster}
							<td>
								<div class="flex flex-col gap-1">
								{#each Object.values(entry.services) as service}
									{#if service.arn.includes(cluster.name)}										
										<ServiceCell service={service} />
									{/if}
								{/each}
								</div>
							</td>
						{/each}
						<td>
							<DatabaseCell database={entry.dbs.find(db => db.env == AwsEnv.DEV)} />
						</td>
						<td>
							<DatabaseCell database={entry.dbs.find(db => db.env == AwsEnv.DEMO)} />
						</td>
						<td>
							<DatabaseCell database={entry.dbs.find(db => db.env == AwsEnv.PROD)} />
						</td>	
					</tr>
					{/each}
				</tbody>
			</table>
			<hr />
		{/if}
	</div>
	{#if homeEntries.length == 0}
		<h1 class="text-center text-lg">
			Nothing here. Visit Services & Databases tabs and start things you want to track
			from each environemnt. 👻 
		</h1>
	{/if}

	<div class="flex flex-row gap-2">
		<table class="table w-full table-zebra table-compact">
			<thead class="sticky top-0">
				<tr>
					<th>
						<div class="flex gap-2">APP</div>
					</th>
					
					{#each clustersFiltered as cluster}
						<th class="w-40">{cluster.name}</th>
					{/each}
					
					<th class="w-40">{AwsEnv.DEV}</th>
					<th class="w-40">{AwsEnv.DEMO}</th>
					<th class="w-40">{AwsEnv.PROD}</th>
				</tr>
			</thead>
			<tbody class="overflow-y-auto max-h-96">
				{#each homeEntries as entry}
					<tr>
						<td>
							<span class="font-bold flex flex-row align-middle gap-1">
								<button
										on:click={() => {
											userStore.favoriteTrackedName(entry.tracked_name);
										}}
									>
										<StarIcon state={true} />
								</button>
								{entry.tracked_name}
							</span>
						</td>						
						{#each clustersFiltered as cluster}
							<td>
								<div class="flex flex-col gap-1">
									{#each Object.values(entry.services) as service}
										{#if service.arn.includes(cluster.arn.split("/")[1])}
											
												<ServiceCell service={service} />
										{/if}
									{/each}
								</div>
							</td>
						{/each}
						<td>
							<DatabaseCell database={entry.dbs.find(db => db.env == AwsEnv.DEV)} />
						</td>
						<td>
							<DatabaseCell database={entry.dbs.find(db => db.env == AwsEnv.DEMO)} />
						</td>
						<td>
							<DatabaseCell database={entry.dbs.find(db => db.env == AwsEnv.PROD)} />
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
</div>
