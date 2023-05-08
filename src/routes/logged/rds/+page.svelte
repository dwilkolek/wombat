<script lang="ts">
	import dbeaver from '$lib/images/dbeaver-head.png';
	import { AwsEnv, type DbInstance } from '$lib/types';
	import { userStore } from '$lib/user-store';
	import { envStore } from '$lib/env-store';
	import { taskStore } from '$lib/task-store';
	import { execute } from '$lib/error-store';
	import StarIcon from '$lib/star-icon.svelte';
	import { listen } from '@tauri-apps/api/event';
	import DbSecretBtn from '$lib/db-secret-btn.svelte';

	let arnFilter = '';
	$: user = $userStore;
	$: isFavourite = (name: string): boolean => {
		return !!user.tracked_names.find((tracked_name) => tracked_name == name);
	};
	$: currentEnv = envStore.currentEnv;
	$: databases = execute<DbInstance[]>('databases', { env: $currentEnv }, true);
	$: listen('cache-refreshed', () => {
		databases = execute<DbInstance[]>('databases', { env: $currentEnv }, true);
	});
	$: matchesFilter = (databse: DbInstance): boolean => {
		return arnFilter === '' || databse.arn.toLowerCase().indexOf(arnFilter.toLowerCase()) > 0;
	};
	let envs = Object.keys(AwsEnv);
</script>

<svelte:head>
	<title>RDS</title>
	<meta name="description" content="Wombat" />
</svelte:head>
<div class="my-4 p-2 pb-5">
	<select class="select select-bordered" bind:value={$currentEnv}>
		{#each envs as env}
			<option value={env}>{env}</option>
		{/each}
	</select>
</div>
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
							autocorrect="off"
							autocapitalize="off"
							spellcheck="false"
							placeholder="Looking for something?"
							class="input input-bordered w-full max-w-xs input-xs"
							bind:value={arnFilter}
						/>
					</div>
				</th>
				<td>Engine</td>
				<td colspan="2">Proxy</td>
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
											userStore.favoriteTrackedName(db.name);
										}}
									>
										<StarIcon state={isFavourite(db.name)} />
									</button>

									<div class="flex flex-col">
										<span class="font-bold">{db.name}</span>
										<span class="text-xs">{db.arn}</span>
										<span class="text-xs">{db.endpoint.address}:{db.endpoint.port}</span>
									</div>
								</div>
							</td>
							<td>
								<span>
									<DbSecretBtn database={db} />
									{db.engine}
								</span>
							</td>
							<td>
								{#if $userStore.dbeaver_path}
									<button
										class={`btn btn-circle ${
											!$taskStore.find((t) => t.arn == db.arn) ? 'opacity-25' : ''
										}`}
										disabled={!$taskStore.find((t) => t.arn == db.arn)}
										on:click={() => {
											execute(
												'open_dbeaver',
												{
													db,
													port: $taskStore.find((t) => t.arn == db.arn)?.port
												},
												false
											);
										}}
									>
										<img width="48" src={dbeaver} alt="download icon" />
									</button>
								{/if}
							</td>

							<td>
								{#if !$taskStore.find((t) => t.arn == db.arn)}
									<button
										class="btn btn-focus"
										disabled={!!$taskStore.find((t) => t.arn == db.arn)}
										on:click={() => {
											execute('start_db_proxy', { db });
										}}>START PROXY</button
									>
								{/if}
								{#if $taskStore.find((t) => t.arn == db.arn)}
									Running on port: {$taskStore.find((t) => t.arn == db.arn)?.port}
								{/if}
							</td>
						</tr>
					{/if}
				{/each}
			{/await}
		</tbody>
	</table>
</div>
