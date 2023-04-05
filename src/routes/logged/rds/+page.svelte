<script lang="ts">
	import Icon from 'svelte-icon/Icon.svelte';
	import { invoke } from '@tauri-apps/api/tauri';
	import star from '$lib/images/star-solid.svg?raw';
	import type { DbInstance, EcsService } from '$lib/types';
	import { userStore } from '$lib/user-store';
	import { envStore } from '$lib/env-store';
	import { taskStore } from '$lib/task-store';

	let arnFilter = '';
	$: user = $userStore;
	$: isFavourite = (serviceName: string): boolean => {
		return !!user.favourite_service_names.find((s) => s == serviceName);
	};
	$: currentEnv = envStore.currentEnv;
	$: databases = invoke<DbInstance[]>('databases', { env: $currentEnv });
	$: matchesFilter = (databse: DbInstance): boolean => {
		return arnFilter === '' || databse.arn.indexOf(arnFilter) > 0;
	};
	$: console.log('tasks', $taskStore);
</script>

<svelte:head>
	<title>RDS</title>
	<meta name="description" content="Wombat" />
</svelte:head>
<div class="h-full block">
	<table class="table w-full table-zebra table-compact">
		<thead class="sticky top-0">
			<tr>
				<th colspan="3">
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
			</tr>
		</thead>
		<tbody class="overflow-y-auto max-h-96">
			{#await databases then databases}
				{#each databases as db, i}
					{#if matchesFilter(db)}
						<tr>
							<td>
								<div class="flex flex-row items-stretch gap-1">
									<button
										on:click={() => {
											userStore.favoriteService(db.name);
										}}
									>
										<Icon
											data={star}
											size="2.2em"
											fill={isFavourite(db.name) ? 'yellow' : 'accent'}
											stroke={isFavourite(db.name) ? 'yellow' : 'accent'}
										/>
									</button>

									<div class="flex flex-col">
										<span class="font-bold">{db.name}</span>
										<span class="text-xs">{db.arn}</span>
										<span class="text-xs">{db.endpoint.address}:{db.endpoint.port}</span>
									</div>
								</div>
							</td>
							<td>
								<button
									class="btn btn-focus"
									disabled={!$taskStore.find((t) => t.arn == db.arn)}
									on:click={() => {
										invoke('open_dbeaver', {
											db,
											port: $taskStore.find((t) => t.arn == db.arn).port
										});
									}}>Open in dbeaver</button
								>
							</td>

							<td>
								<button
									class="btn btn-focus"
									disabled={$taskStore.find((t) => t.arn == db.arn)}
									on:click={() => {
										invoke('start_db_proxy', { db, port: 10000 });
									}}>{$taskStore.find((t) => t.arn == db.arn) ? 'RUNNING' : 'START PROXY'}</button
								>
							</td>
						</tr>
					{/if}
				{/each}
			{/await}
		</tbody>
	</table>
</div>
