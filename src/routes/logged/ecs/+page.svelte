<script lang="ts">
	import Icon from 'svelte-icon/Icon.svelte';
	import { invoke } from '@tauri-apps/api/tauri';
	import star from '$lib/images/star-solid.svg?raw';
	import type { EcsService } from '$lib/types';
	import { userStore } from '$lib/user-store';
	import { envStore } from '$lib/env-store';
	import { execute } from '$lib/error-store';
	import { taskStore } from '$lib/task-store';
	import { open } from '@tauri-apps/api/shell';

	let arnFilter = '';
	$: user = $userStore;
	$: isFavourite = (arn: string): boolean => {
		return !!user.ecs.find((ecsArn) => ecsArn == arn);
	};
	$: activeCluser = envStore.activeCluser;

	$: services = $activeCluser
		? execute<EcsService[]>('services', { cluster: $activeCluser }, true)
		: [];
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
							autocomplete="false"
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
			{#await services then services}
				{#each services as service, i}
					{#if matchesFilter(service)}
						<tr>
							<td>
								<div class="flex flex-row items-stretch gap-1">
									<button
										on:click={() => {
											userStore.favoriteEcs(service.arn);
										}}
									>
										<Icon
											data={star}
											size="2.2em"
											fill={isFavourite(service.arn) ? 'yellow' : 'accent'}
											stroke={isFavourite(service.arn) ? 'yellow' : 'accent'}
										/>
									</button>

									<div class="flex flex-col">
										<span class="font-bold"> {service.name}</span>
										<span class="text-xs"> {service.arn}</span>
									</div>
								</div>
							</td>
							<td>
								<div class="flex flex-col">
									{#if !$taskStore.find((t) => t.arn == service.arn)}
										<button
											class="btn btn-focus"
											on:click={() => {
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
