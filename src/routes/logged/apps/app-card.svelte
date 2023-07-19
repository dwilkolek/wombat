<script lang="ts">
	import { execute } from '$lib/error-store';
	import { taskStore } from '$lib/task-store';
	import { AwsEnv, type DbInstance } from '$lib/types';
	import { userStore } from '$lib/user-store';
	import DbSecretBtn from '$lib/db-secret-btn.svelte';
	import { ask } from '@tauri-apps/api/dialog';

	import { serviceDetailStore } from '$lib/service-details-store';
	import DatabaseCell from './database-cell.svelte';
	import ServiceCell from './service-cell.svelte';
	export let app: string;

	$: detailsStorr = serviceDetailStore(app);
	$: details = $detailsStorr;
</script>

<div class="card card-compact w-96 bg-base-100 shadow-xl">
	<div class="card-body">
		<h2 class="card-title">{app}</h2>
		{#if !details}
			<span class="loading loading-dots loading-lg" />
		{/if}
		{#if details}
			{#each [...details.envs] as [env, value]}
				<div class="flex flex-row gap-2">
					<div class="font-bold">{env}:</div>
					{#each value.services as service}
						<ServiceCell {service} />
					{/each}
					{#each value.dbs as db}
						<DatabaseCell database={db} />
					{/each}
				</div>
			{/each}
		{/if}
	</div>
</div>
