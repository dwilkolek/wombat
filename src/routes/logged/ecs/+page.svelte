<script lang="ts">
	import { invoke } from '@tauri-apps/api/tauri';
	import { readable, writable } from 'svelte/store';
	import type { EcsService } from '../../types';
	let clustersPromise = invoke<string[]>('clusters');
	let selectedClusterArn = writable<string | undefined>();
	let services = readable<EcsService[]>([]);

	clustersPromise.then(async (clusters) => {
		selectedClusterArn.set(clusters[0]);
	});
	selectedClusterArn.subscribe(async (clusterArn) => {
		services = readable(await invoke<EcsService[]>('services', { clusterArn }));
	});
</script>

<svelte:head>
	<title>Home</title>
	<meta name="description" content="Wombat" />
</svelte:head>

<div class="mx-auto">
	<div class="overflow-x-auto">
		{#await clustersPromise}
			<p>loading</p>
		{:then clusters}
			<div>
				<select class="select select-bordered" bind:value={$selectedClusterArn}>
					{#each clusters as clusterArn}
						<option value={clusterArn}>{clusterArn}</option>
					{/each}
				</select>
				<table class="table w-full table-zebra table-compact">
					<thead>
						<tr>
							<th />
							<th>Name</th>
							<th>Monitor</th>
						</tr>
					</thead>
					<tbody>
						{#each $services as { name, service_arn }, i}
							<tr>
								<th>{i}</th>
								<td>
									<div class="flex flex-col">
										<span class="font-bold"> {name}</span>
										<span class="text-xs"> {service_arn}</span>
									</div>
								</td>
								<td>
									<button
										class="btn btn-focus"
										on:click={async () => {
											await invoke('monitor_service', { serviceArn: service_arn });
										}}>Monitor</button
									>
								</td>
							</tr>
						{/each}
					</tbody>
				</table>
			</div>
		{:catch error}
			<p style="color: red">{error}</p>
		{/await}
	</div>
</div>
