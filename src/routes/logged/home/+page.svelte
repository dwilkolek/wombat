<script lang="ts">
	import { AwsEnv } from '$lib/types';
	import { homeStore, type HomeEntry } from '$lib/home-store';
	import { discoverStore } from '$lib/discover-store';
	import DatabaseCell from './database-cell.svelte';
	import ServiceCell from './service-cell.svelte';
	import { userStore } from '$lib/user-store';
	import { clusterStore } from '$lib/cluster-store';
	import StarIcon from '$lib/star-icon.svelte';
	type ListType = HomeEntry & { id: string };
	$: homeStore.init();
	$: homeEntries = $homeStore
		? $homeStore.sort((a, b) => a.tracked_name.localeCompare(b.tracked_name))
		: [];
	$: clusters = clusterStore.clusters;
	$: selectedClusters = $userStore.preffered_environments;
	$: allEntries = [
		...($discoverStore ?? []).map((e) => ({
			...e,
			id: `proposed#${e.tracked_name}`
		})),
		($discoverStore?.length ?? 0) > 0 ? { id: `break` } : undefined,
		...homeEntries.map((e) => ({
			...e,
			id: `${e.tracked_name}`
		}))
	].filter((e) => !!e) as ListType[];
	$: columnToggleHandler = (env: AwsEnv, e: any) => {
		if (!e.currentTarget.checked) {
			userStore.savePrefferedEnvs([
				...selectedClusters.filter((selectedEnv) => env != selectedEnv)
			]);
		} else {
			userStore.savePrefferedEnvs([...selectedClusters, env]);
		}
	};

	const envs = [AwsEnv.PLAY, AwsEnv.LAB, AwsEnv.DEV, AwsEnv.DEMO, AwsEnv.PROD];
	let discoverValue: string = '';
</script>

<svelte:head>
	<title>HOME</title>
	<meta name="description" content="Wombat" />
</svelte:head>
<div class="h-full block">
	<div class="my-4 p-2 pb-5 flex flex-row justify-between">
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
		<div class="flex flex-row flex-wrap gap-5">
			{#each envs as env (env)}
				<div class="form-control">
					<!-- svelte-ignore a11y-click-events-have-key-events -->
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
	{#if homeEntries.length == 0}
		<h1 class="text-center text-lg">
			Nothing here. Visit Services & Databases tabs and start things you want to track from each
			environemnt. 👻
		</h1>
	{/if}

	<div class="flex flex-row gap-2">
		<table class="table border-separate w-full table-zebra table-compact">
			<thead class="">
				<tr>
					<th>
						<div class="flex gap-2">APP</div>
					</th>
					{#each envs as env}
						{#each $clusters.filter((c) => c.env == env && selectedClusters.includes(c.env)) as cluster, i}
							<th class={i == 0 ? 'border-l-2 w-40' : 'w-40'}>{cluster.name}</th>
						{/each}
						{#if selectedClusters.includes(env)}
							<th class="w-40"> Database@{env}</th>
						{/if}
					{/each}
				</tr>
			</thead>
			<tbody class="overflow-y-auto max-h-96">
				{#if $discoverStore && $discoverStore.length == 0}
					<tr>
						<td colspan="4">Nothing new was found</td>
					</tr>
				{/if}

				{#each allEntries as entry (entry.id)}
					{#if entry.id === 'break'}
						<tr>
							<td colspan={$clusters.length + 4}>
								<hr />
							</td>
						</tr>
					{/if}
					{#if entry.id !== 'break'}
						<tr class="hover">
							<td>
								<div class="font-bold flex flex-row align-middle items-center gap-1">
									<button
										on:click={() => {
											userStore.favoriteTrackedName(entry.tracked_name);
										}}
									>
										<StarIcon state={$userStore.tracked_names.includes(entry.tracked_name)} />
									</button>
									<span>{entry.tracked_name}</span>
								</div>
							</td>
							{#each envs as env}
								{#each $clusters.filter((c) => c.env == env && selectedClusters.includes(c.env)) as cluster, i}
									<td class={i == 0 ? 'border-l-2' : ''}>
										<div class="flex flex-col gap-1">
											{#each Object.values(entry.services) as service}
												{#if service.arn.includes(cluster.name)}
													<ServiceCell {service} />
												{/if}
											{/each}
										</div>
									</td>
									<td>
										<DatabaseCell database={entry.dbs.find((db) => db.env == env)} />
									</td>
								{/each}
							{/each}
						</tr>
					{/if}
				{/each}
			</tbody>
		</table>
	</div>
</div>
