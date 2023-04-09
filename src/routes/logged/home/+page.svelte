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
	$: homeStore.init();
	$: keys = $homeStore ? Object.keys($homeStore).sort((a, b) => a.localeCompare(b)) : [];

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
	<div class="flex flex-row gap-2">
		<table class="table w-full table-zebra table-compact">
			<thead class="sticky top-0">
				<tr>
					<th>
						<div class="flex gap-2">Service</div>
					</th>
					<th class="w-40">ECS DEV</th>
					<th class="w-40">ECS DEMO</th>
					<th class="w-40">ECS PROD</th>
					<th class="w-40">RDS DEV</th>
					<th class="w-40">RDS DEMO</th>
					<th class="w-40">RDS PROD</th>
					<th class="w-10" />
				</tr>
			</thead>
			<tbody class="overflow-y-auto max-h-96">
				{#if keys.length === 0}
					<tr>
						<td colspan="8">
							<h1 class="text-center text-lg">
								Nothing here. Visit Services & Databases tabs and start things you want to track
								from each environemnt. 👻
							</h1>
						</td>
					</tr>
				{/if}
				{#each keys as key}
					<tr>
						<td>
							<span class="font-bold flex flex-row align-middle gap-1">
								{key}
							</span>
						</td>
						<td class="align-top">
							<ServiceCell service={$homeStore[key][AwsEnv.DEV]?.service} />
						</td>
						<td class="align-top">
							<ServiceCell service={$homeStore[key][AwsEnv.DEMO]?.service} />
						</td>
						<td class="align-top">
							<ServiceCell service={$homeStore[key][AwsEnv.PROD]?.service} />
						</td>
						<td class="align-top">
							<DatabaseCell database={$homeStore[key][AwsEnv.DEV]?.db} />
						</td>
						<td class="align-top">
							<DatabaseCell database={$homeStore[key][AwsEnv.DEMO]?.db} />
						</td>
						<td class="align-top">
							<DatabaseCell database={$homeStore[key][AwsEnv.PROD]?.db} />
						</td>
						<td>
							<span class="font-bold flex flex-row align-middle gap-1">
								<button
									on:click={() => {
										discoverValue = key;
										discoverStore.discover(key);
									}}
									><svg
										xmlns="http://www.w3.org/2000/svg"
										viewBox="0 0 20 20"
										fill="currentColor"
										class="w-5 h-5"
									>
										<path d="M6.5 9a2.5 2.5 0 115 0 2.5 2.5 0 01-5 0z" />
										<path
											fill-rule="evenodd"
											d="M10 18a8 8 0 100-16 8 8 0 000 16zM9 5a4 4 0 102.248 7.309l1.472 1.471a.75.75 0 101.06-1.06l-1.471-1.472A4 4 0 009 5z"
											clip-rule="evenodd"
										/>
									</svg>
								</button>
								<button
									on:click={() => {
										for (let entry of Object.values($homeStore[key])) {
											if (entry.db) {
												userStore.favoriteRds(entry.db.arn);
											}
											if (entry.service) {
												userStore.favoriteEcs(entry.service.arn);
											}
										}
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
