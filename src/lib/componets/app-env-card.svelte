<script lang="ts">
	import { serviceDetailStore } from '$lib/stores/service-details-store';
	import DatabaseCell from './database-cell.svelte';
	import ServiceCell from './service-cell.svelte';
	import type { AwsEnv } from '$lib/types';
	interface Props {
		app: string;
		env: AwsEnv;
	}

	let { app, env }: Props = $props();

	let details = serviceDetailStore(app);
	let envDetails = $derived($details?.envs?.get(env));
</script>

{#if envDetails}
	<div class="p-2">
		<div class="card card-compact w-full bg-base-100 shadow-xl">
			<div class="card-body">
				<div class="card-title flex flex-row gap-2 items-center text-md">
					{env}
				</div>
				<div class="flex flex-col gap-2 text-md">
					<div class="">
						<h4 class="font-medium text-lg">Service:</h4>
						<div class="flex">
							{#each envDetails.services as service}
								<div class="flex flex-col gap-2">
									<h5>ARN: {service.arn}</h5>
									<span>Task version: {service.version}</span>
									<div class="flex flex-row gap-2 items-center">
										<span>Proxy:</span>
										<ServiceCell {service} />
									</div>
								</div>
							{/each}
						</div>
					</div>
					<div class="">
						<h4 class="font-medium text-lg">Database:</h4>
						<div class="flex flex-col gap-2">
							{#each envDetails.dbs as db}
								<div class="flex flex-col">
									<h5>ARN: {db.arn}</h5>
									<span>Engine: {db.engine}</span>
									<span>Engine Version:{db.engine_version}</span>
									<div class="flex flex-row gap-2 items-center">
										<span>Proxy:</span>
										<DatabaseCell database={db} />
									</div>
								</div>
							{/each}
						</div>
					</div>
				</div>
			</div>
		</div>
	</div>
{/if}
