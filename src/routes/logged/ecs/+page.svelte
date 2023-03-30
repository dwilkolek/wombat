<script lang="ts">
	import Icon from 'svelte-icon/Icon.svelte';
	import { invoke } from '@tauri-apps/api/tauri';
	import { onMount } from 'svelte';
	import star from '$lib/images/star-solid.svg?raw';
	import type { EcsService } from '../../types';
	import { userStore } from '../../user-store';
	let clusterArns: string[] = [];
	let servicesArns: EcsService[] = [];
	let arnFilter = '';
	let selectedArn: string = '';
	$: user = $userStore;
	$: isFavourite = (serviceName: string): boolean => {
		return !!user.favourite_service_names.find((s) => s == serviceName);
	};
	onMount(async () => {
		clusterArns = await invoke<string[]>('clusters');
		selectedArn = clusterArns[0];
		servicesArns = await invoke<EcsService[]>('services', { clusterArn: selectedArn });
	});
	$: filtered = servicesArns.filter((v) => {
		return arnFilter === '' || v.service_arn.indexOf(arnFilter) > 0;
	});
</script>

<svelte:head>
	<title>Home</title>
	<meta name="description" content="Wombat" />
</svelte:head>

<div class="mx-auto">
	<div class="overflow-x-auto">
		<div>
			<div class="flex gap-2">
				<select class="select select-bordered" bind:value={selectedArn}>
					{#each clusterArns as clusterArn}
						<option value={clusterArn}>{clusterArn}</option>
					{/each}
				</select>
				<input
					type="text"
					placeholder="Looking for something?"
					class="input input-bordered w-full max-w-xs"
					bind:value={arnFilter}
				/>
			</div>
			<table class="table w-full table-zebra table-compact">
				<thead>
					<tr>
						<th />
						<th>Name</th>
						<th>Monitor</th>
					</tr>
				</thead>
				<tbody>
					{#each filtered as { name, service_arn }, i}
						<tr>
							<th>{i}</th>
							<td>
								<div class="flex flex-row items-stretch gap-1">
									<button
										on:click={() => {
											userStore.favoriteService(name);
										}}
									>
										<Icon
											data={star}
											size="2.2em"
											fill={isFavourite(name) ? 'yellow' : 'accent'}
											stroke={isFavourite(name) ? 'yellow' : 'accent'}
										/>
									</button>

									<div class="flex flex-col">
										<span class="font-bold"> {name}</span>
										<span class="text-xs"> {service_arn}</span>
									</div>
								</div>
							</td>
							<td>
								<button class="btn btn-focus" disabled={true}>Proxy</button>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		</div>
	</div>
</div>
