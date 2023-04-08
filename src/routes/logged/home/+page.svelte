<script lang="ts">
	import { AwsEnv } from '$lib/types';
	import { homeStore } from '$lib/home-store';
	import { execute } from '$lib/error-store';
	import { taskStore } from '$lib/task-store';
	import DatabaseCell from './database-cell.svelte';
	import ServiceCell from './service-cell.svelte';
	$: homeStore.init();
	$: entries = $homeStore;
	$: keys = entries ? Object.keys(entries) : [];
</script>

<svelte:head>
	<title>HOME</title>
	<meta name="description" content="Wombat" />
</svelte:head>
<div class="h-full block">
	<div class="flex flex-row gap-2">
		<table class="table w-full table-zebra table-compact">
			<thead class="sticky top-0">
				<tr>
					<th>
						<div class="flex gap-2">Service</div>
					</th>
					<th class="w-40">ECS DEV</th>
					<th class="w-40">ECS DEMO</th>
					<th class="w-40">ECS PROD</th>
					<th class="w-40">RDS DEV</th>
					<th class="w-40">RDS DEMO</th>
					<th class="w-40">RDS PROD</th>
				</tr>
			</thead>
			<tbody class="overflow-y-auto max-h-96">
				{#if keys.length === 0}
					<tr>
						<td colspan="7">
							<h1 class="text-center text-lg">
								Nothing here. Visit Services & Databases tabs and start things you want to track
								from each environemnt. 👻
							</h1>
						</td>
					</tr>
				{/if}
				{#each keys as key}
					<tr>
						<td>
							<span class="font-bold flex flex-row align-middle gap-1">
								{key}
								<!-- <button
									on:click={() => {
										homeStore.discover(key);
									}}
									><svg
										xmlns="http://www.w3.org/2000/svg"
										viewBox="0 0 20 20"
										fill="currentColor"
										class="w-5 h-5"
									>
										<path d="M6.5 9a2.5 2.5 0 115 0 2.5 2.5 0 01-5 0z" />
										<path
											fill-rule="evenodd"
											d="M10 18a8 8 0 100-16 8 8 0 000 16zM9 5a4 4 0 102.248 7.309l1.472 1.471a.75.75 0 101.06-1.06l-1.471-1.472A4 4 0 009 5z"
											clip-rule="evenodd"
										/>
									</svg>
								</button> -->
							</span>
						</td>
						<td class="align-top">
							<ServiceCell service={entries[key][AwsEnv.DEV]?.service} />
						</td>
						<td class="align-top">
							<ServiceCell service={entries[key][AwsEnv.DEMO]?.service} />
						</td>
						<td class="align-top">
							<ServiceCell service={entries[key][AwsEnv.PROD]?.service} />
						</td>
						<td class="align-top">
							<DatabaseCell database={entries[key][AwsEnv.DEV]?.db} />
						</td>
						<td class="align-top">
							<DatabaseCell database={entries[key][AwsEnv.DEMO]?.db} />
						</td>
						<td class="align-top">
							<DatabaseCell database={entries[key][AwsEnv.PROD]?.db} />
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
</div>
