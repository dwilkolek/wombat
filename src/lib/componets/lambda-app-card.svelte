<script lang="ts">
	import { wombatProfileStore } from '$lib/stores/available-profiles-store';
	import { taskStore } from '$lib/stores/task-store';
	import AppCardHr from './app-card-hr.svelte';
	import LambdaTaskStatus from './lambda-task-status.svelte';
	import LambdaAppProxyBtn from './lambda-app-proxy-btn.svelte';
	export let app: string;

	$: tasks = $taskStore;
</script>

<div class="px-2 py-1 shadow-2xl w-full flex rounded-lg bg-base-300">
	<div class="flex gap-2 flex-col justify-around">
		<div class="min-w-80 w-80 flex flex-row gap-2 items-center text-md">
			<span class="inline text-base">
				{app}
			</span>
		</div>
	</div>

	<div
		class={`grid w-full divide-x divide-base-100`}
		style={`grid-template-columns: repeat(${$wombatProfileStore.environments.length ?? 1}, minmax(0, 1fr));`}
	>
		{#each $wombatProfileStore.environments as enabled_env}
			{@const task = tasks.find(
				(task) => task.arn == `lambdaApp::${app}::${enabled_env.toLowerCase()}`
			)}
			<div class={`flex flex-col app-env-cell px-2`}>
				<div class="font-medium text-xs italic flex gap-1 items-center">
					<span>{enabled_env}:</span>
				</div>
				<div class="flex gap-1 app-env-cell-stack">
					<div class="flex flex-row items-center gap-1 px-1">
						<LambdaAppProxyBtn {app} env={enabled_env} />
						<div class="flex gap-2 justify-between items-center grow">
							<span class="truncate"></span>
							<AppCardHr {task} />
							<LambdaTaskStatus {task} {app} env={enabled_env} />
						</div>
					</div>
				</div>
			</div>
		{/each}
	</div>
</div>

<style>
	.app-env-cell-stack {
		flex-direction: column;
	}
</style>
