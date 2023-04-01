<script lang="ts">
	import Icon from 'svelte-icon/Icon.svelte';
	import { invoke } from '@tauri-apps/api/tauri';
	import star from '$lib/images/star-solid.svg?raw';
	import type { EcsService } from '../../types';
	import { userStore } from '../../user-store';
	import { currentEnv } from '../../env-store';

	let arnFilter = '';
	$: user = $userStore;
	$: isFavourite = (serviceName: string): boolean => {
		return !!user.favourite_service_names.find((s) => s == serviceName);
	};
	$: services = invoke<EcsService[]>('services', { env: $currentEnv });
	$: matchesFilter = (service: EcsService): boolean => {
		return arnFilter === '' || service.arn.indexOf(arnFilter) > 0;
	};
</script>

<svelte:head>
	<title>ECS</title>
	<meta name="description" content="Wombat" />
</svelte:head>
<div class="h-full block">
	<table class="table w-full table-zebra table-compact">
		<thead class="sticky top-0">
			<tr>
				<th>
					<div class="flex gap-2">
						Info
						<input
							type="text"
							placeholder="Looking for something?"
							class="input input-bordered w-full max-w-xs input-xs"
							bind:value={arnFilter}
						/>
					</div>
				</th>
				<th class="w-40">Monitor</th>
			</tr>
		</thead>
		<tbody class="overflow-y-auto max-h-96">
			{#await services then services}
				{#each services as service, i}
					{#if matchesFilter(service)}
						<tr>
							<td>
								<div class="flex flex-row items-stretch gap-1">
									<button
										on:click={() => {
											userStore.favoriteService(service.name);
										}}
									>
										<Icon
											data={star}
											size="2.2em"
											fill={isFavourite(service.name) ? 'yellow' : 'accent'}
											stroke={isFavourite(service.name) ? 'yellow' : 'accent'}
										/>
									</button>

									<div class="flex flex-col">
										<span class="font-bold"> {service.name}</span>
										<span class="text-xs"> {service.arn}</span>
									</div>
								</div>
							</td>
							<td>
								<button class="btn btn-focus" disabled={true}>Proxy</button>
							</td>
						</tr>
					{/if}
				{/each}
			{/await}
		</tbody>
	</table>
</div>
