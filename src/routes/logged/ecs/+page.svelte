<script lang="ts">
	import { invoke } from '@tauri-apps/api/tauri';
	import { state, type EcsService } from '../../store';
	let records: EcsService[] = [];
	state.env.subscribe(async () => {
		records = await invoke<[]>('services');
	});
</script>

<svelte:head>
	<title>Home</title>
	<meta name="description" content="Wombat" />
</svelte:head>
<div class="mx-auto">
	<div class="overflow-x-auto">
		<table class="table w-full table-zebra table-compact">
			<thead>
				<tr>
					<th />
					<th>Name</th>
					<th>Environment</th>
				</tr>
			</thead>
			<tbody>
				{#each records as record, i}
					<tr>
						<th>{i}</th>
						<td>
							<div class="flex flex-col">
								<span class="font-bold"> {record.name}</span>
								<span class="text-xs"> {record.service_arn}</span>
							</div>
						</td>
						<td>{record.env}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
</div>
