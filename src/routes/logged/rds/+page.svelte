<script lang="ts">
	import { invoke } from '@tauri-apps/api/tauri';
	import { state, type DbInstance } from '../../store';
	let records: DbInstance[] = [];
	state.env.subscribe(async () => {
		records = await invoke<[]>('databases');
	});
</script>

<svelte:head>
	<title>Home</title>
	<meta name="description" content="Wombat" />
</svelte:head>
<div class="mx-auto">
	<div class="overflow-x-auto">
		<table class="table w-full table-zebra table-compact">
			<!-- head -->
			<thead>
				<tr>
					<th />
					<th>Base info</th>
					<th>Service</th>
				</tr>
			</thead>
			<tbody>
				{#each records as record, i}
					<tr>
						<th>{i}</th>

						<td
							><div class="flex flex-col">
								<span class="font-bold">{record.db_name}</span>
								<span class="text-xs">{record.db_instance_arn}</span>
								<span class="text-xs">{record.endpoint.address}:{record.endpoint.port}</span>
							</div>
						</td>
						<td>{record.service}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
</div>
