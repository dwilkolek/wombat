<script lang="ts">
	import Icon from 'svelte-icon/Icon.svelte';
	import star from '$lib/images/star-solid.svg?raw';
	import { AwsEnv } from '$lib/types';
	import { homeStore } from '$lib/home-store';
	import { discoverStore } from '$lib/discover-store';
	import DatabaseCell from './database-cell.svelte';
	import ServiceCell from './service-cell.svelte';
	import { userStore } from '$lib/user-store';
	import StarIcon from '$lib/star-icon.svelte';
	import { envStore } from '$lib/env-store';
	$: homeStore.init();
	$: homeEntries = $homeStore ? $homeStore.sort((a, b) => a.tracked_name.localeCompare(b.tracked_name)) : [];
	$: clusters = envStore.clusters
	$: usedEnvs = homeEntries.flatMap(e => [...e.dbs.map(db => db.env), ...Object.values(e.services).map(s => s.env)])
	$: clustersFiltered = $clusters.filter(cluster =>usedEnvs.includes(cluster.env));

	let discoverValue: string = '';
</script>

<svelte:head>
	<title>HOME</title>
	<meta name="description" content="Wombat" />
</svelte:head>
<div class="h-full block">
	<div class="my-4 p-2">
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
							<div class="flex gap-2">Type: Name</div>
						</th>
						<th class="w-40">Environemnt</th>
						<th class="">Arn</th>
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
								<div class="font-bold flex flex-row items-center gap-1">
									<span>{entry[0]}: {entry[3]}</span>
									<button
										on:click={() => {
											if ($discoverStore?.length == 1) {
												discoverStore.set(undefined);
												discoverValue = '';
											}
											if (entry[0] == 'Service') {
												userStore.favoriteEcs(entry[2]);
											} else {
												userStore.favoriteRds(entry[2]);
											}
										}}
									>
										<StarIcon state={false} />
									</button>
								</div>
							</td>
							<td class="align-top">
								{entry[1]}
							</td>
							<td class="align-top">
								{entry[2]}
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
						<div class="flex gap-2">Service</div>
					</th>
					{#each clustersFiltered as cluster}
						<th class="w-40">{cluster.arn.split("/")[1]}</th>
					{/each}
					<th class="w-10" />
				</tr>
			</thead>
			<tbody class="overflow-y-auto max-h-96">
				{#each homeEntries as entry}
					<tr>
						<td>
							<span class="font-bold flex flex-row align-middle gap-1">
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
								{#each entry.dbs as db}
										{#if db.env == cluster.env}
												<DatabaseCell database={db} />
										{/if}
									{/each}
								</div>
							</td>
						{/each}
						<td>
							<span class="font-bold flex flex-row align-middle gap-1">
								<button
									on:click={() => {
										userStore.favoriteTrackedName(entry.tracked_name)
										
									}}
									><svg
										xmlns="http://www.w3.org/2000/svg"
										viewBox="0 0 20 20"
										fill="currentColor"
										class="w-5 h-5"
									>
										<path d="M6.5 9a2.5 2.5 0 115 0 2.5 2.5 0 01-5 0z" />
										<svg
											xmlns="http://www.w3.org/2000/svg"
											fill="none"
											viewBox="0 0 24 24"
											stroke-width="1.5"
											stroke="currentColor"
											class="w-6 h-6"
										>
											<svg
												xmlns="http://www.w3.org/2000/svg"
												viewBox="0 0 20 20"
												fill="currentColor"
												class="w-5 h-5"
											>
												<path
													fill-rule="evenodd"
													d="M8.75 1A2.75 2.75 0 006 3.75v.443c-.795.077-1.584.176-2.365.298a.75.75 0 10.23 1.482l.149-.022.841 10.518A2.75 2.75 0 007.596 19h4.807a2.75 2.75 0 002.742-2.53l.841-10.52.149.023a.75.75 0 00.23-1.482A41.03 41.03 0 0014 4.193V3.75A2.75 2.75 0 0011.25 1h-2.5zM10 4c.84 0 1.673.025 2.5.075V3.75c0-.69-.56-1.25-1.25-1.25h-2.5c-.69 0-1.25.56-1.25 1.25v.325C8.327 4.025 9.16 4 10 4zM8.58 7.72a.75.75 0 00-1.5.06l.3 7.5a.75.75 0 101.5-.06l-.3-7.5zm4.34.06a.75.75 0 10-1.5-.06l-.3 7.5a.75.75 0 101.5.06l.3-7.5z"
													clip-rule="evenodd"
												/>
											</svg>
										</svg>
									</svg>
								</button>
							</span>
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
</div>
