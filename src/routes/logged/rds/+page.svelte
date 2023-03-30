<script lang="ts">
	import { invoke } from '@tauri-apps/api/tauri';
	import type { DbInstance } from '../../types';
	import { userStore } from '../../user-store';
	import star from '$lib/images/star-solid.svg?raw';
	import Icon from 'svelte-icon/Icon.svelte';
	let databasesPromise = invoke<DbInstance[]>('databases');

	$: user = $userStore;
	$: isFavourite = (dbArn: string): boolean => {
		return !!user?.favourite_db_arns?.find((s) => s == dbArn);
	};
</script>

<svelte:head>
	<title>RDS</title>
	<meta name="description" content="Wombat" />
</svelte:head>
<div class="mx-auto">
	<div class="overflow-x-auto">
		{#await databasesPromise}
			<p>loading</p>
		{:then databases}
			<table class="table w-full table-zebra table-compact">
				<!-- head -->
				<thead>
					<tr>
						<th />
						<th>Base info</th>
					</tr>
				</thead>
				<tbody>
					{#each databases as record, i}
						<tr>
							<th>{i}</th>

							<td>
								<div class="flex flex-row items-stretch gap-1">
									<button
										on:click={() => {
											userStore.favoriteDb(record.db_instance_arn);
										}}
									>
										<Icon
											data={star}
											size="2.2em"
											fill={isFavourite(record.db_instance_arn) ? 'yellow' : 'accent'}
											stroke={isFavourite(record.db_instance_arn) ? 'yellow' : 'accent'}
										/>
									</button>

									<div class="flex flex-col">
										<span class="font-bold">{record.db_name}</span>
										<span class="text-xs">{record.db_instance_arn}</span>
										<span class="text-xs">{record.endpoint.address}:{record.endpoint.port}</span>
									</div>
								</div>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		{/await}
	</div>
</div>
