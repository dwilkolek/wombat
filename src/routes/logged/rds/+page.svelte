<script lang="ts">
	import { invoke } from '@tauri-apps/api/tauri';
	import type { DbInstance } from '../../types';
	let databasesPromise = invoke<DbInstance[]>('databases');
</script>

<svelte:head>
	<title>Home</title>
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

							<td
								><div class="flex flex-col">
									<span class="font-bold">{record.db_name}</span>
									<span class="text-xs">{record.db_instance_arn}</span>
									<span class="text-xs">{record.endpoint.address}:{record.endpoint.port}</span>
								</div>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		{/await}
	</div>
</div>
