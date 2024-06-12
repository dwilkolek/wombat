<script lang="ts">
	import { invoke } from '@tauri-apps/api/tauri';
	import { AwsEnv, type EcsService } from '$lib/types';
	import { activeProfilePreferences, userStore } from '$lib/stores/user-store';
	import { taskStore } from '$lib/stores/task-store';
	import { open } from '@tauri-apps/api/shell';
	import StarIcon from '$lib/componets/star-icon.svelte';
	import { clusterStore } from '$lib/stores/cluster-store';
	import { serviceStore } from '$lib/stores/service-store';
	import { ask } from '@tauri-apps/api/dialog';

	let arnFilter = '';
	$: isFavourite = (name: string): boolean => {
		return !!$activeProfilePreferences.tracked_names.find((tracked_name) => tracked_name == name);
	};
	$: activeCluser = clusterStore.activeCluser;

	$: services = serviceStore.getServices($activeCluser);
	$: matchesFilter = (service: EcsService): boolean => {
		return arnFilter === '' || service.arn.toLowerCase().indexOf(arnFilter.toLowerCase()) > 0;
	};
	$: clusters = clusterStore.clusters;
</script>

<svelte:head>
	<title>ECS</title>
	<meta name="description" content="Wombat" />
</svelte:head>
<div class="bg-base-100 sticky top-[68px] z-50 px-2">
	<select class="select select-bordered" bind:value={$activeCluser}>
		{#each $clusters as cluster}
			<option value={cluster}>{cluster.name}</option>
		{/each}
	</select>
</div>
<div class="h-full block">
	<table class="table w-full table-zebra table-compact">
		<thead class="bg-base-100 sticky top-[116px] z-50">
			<tr>
				<th>
					<div class="flex gap-2">
						Info
						<input
							type="text"
							autocomplete="off"
							autocorrect="off"
							autocapitalize="off"
							spellcheck="false"
							placeholder="Looking for something?"
							class="input input-bordered w-full max-w-xs input-xs"
							bind:value={arnFilter}
						/>
					</div>
				</th>
				<th class="w-40 pr-2">Proxy</th>
			</tr>
		</thead>
		<tbody class="overflow-y-auto max-h-96">
			{#await services}
				<span class="loading loading-dots loading-lg" />
			{:then services}
				{#each services as service}
					{#if matchesFilter(service)}
						<tr>
							<td>
								<div class="flex flex-row items-stretch gap-1">
									<button
										on:click={() => {
											userStore.favoriteTrackedName(service.name);
										}}
									>
										<StarIcon state={isFavourite(service.name)} />
									</button>

									<div class="flex flex-col">
										<span class="font-medium"> {service.name}</span>
										<span class="text-xs"> {service.arn}</span>
									</div>
								</div>
							</td>
							<td>
								<div class="flex flex-col">
									{#if !$taskStore.find((t) => t.arn == service.arn)}
										<button
											class="btn btn-focus"
											on:click={async () => {
												if (service?.env == AwsEnv.PROD) {
													let response = await ask(
														'Understand the risks before connecting to production service.\nUnauthorized or unintended changes can have severe consequences.\nProceed with care.',
														{
															title: 'Access to PRODUCTION service.',
															okLabel: 'Proceed',
															cancelLabel: 'Abort',
															type: 'warning'
														}
													);
													if (!response) {
														return;
													}
												}
												invoke('start_service_proxy', { service });
											}}>Start proxy</button
										>{/if}
									{#if $taskStore.find((t) => t.arn == service.arn)}
										<button
											class="underline"
											on:click|preventDefault={() => {
												open(
													'http://localhost:' + $taskStore.find((t) => t.arn == service.arn)?.port
												);
											}}
											>Running on port: {$taskStore.find((t) => t.arn == service.arn)?.port}</button
										>
									{/if}
								</div>
							</td>
						</tr>
					{/if}
				{/each}
			{/await}
		</tbody>
	</table>
</div>
