<script lang="ts">
	import { wombatProfileStore } from '$lib/stores/available-profiles-store';
	import { taskStore } from '$lib/stores/task-store';
	import AppCardHr from './app-card-hr.svelte';
	import LambdaTaskStatus from './lambda-task-status.svelte';
	import LambdaAppProxyBtn from './lambda-app-proxy-btn.svelte';
	import { cookieJar } from '$lib/stores/cookie-jar-status';
	interface Props {
		app: string;
	}

	let { app }: Props = $props();

	let tasks = $derived($taskStore);
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
				<div class="font-medium text-xs flex items-row gap-1 items-center">
					{#if $cookieJar.cookieHealth[enabled_env] == 'Ok'}
						<div
							class={`bg-lime-400 h-[8px] w-[8px] rounded tooltip tooltip-top`}
							data-tip={'Cookie is fresh, <5min'}
						></div>
					{/if}
					{#if $cookieJar.cookieHealth[enabled_env] == 'Stale'}
						<div
							class={`bg-amber-300 h-[8px] w-[8px] rounded tooltip tooltip-top`}
							data-tip={'Cookie is stale, >5min'}
						></div>
					{/if}
					{#if $cookieJar.cookieHealth[enabled_env] == 'Old'}
						<div
							class={`bg-red-500 h-[8px] w-[8px] rounded tooltip tooltip-top`}
							data-tip={'Cookie is old, >10min'}
						></div>
					{/if}
					{#if !$cookieJar.cookieHealth[enabled_env]}
						<div
							class={`bg-gray-500 h-[8px] w-[8px] rounded tooltip tooltip-top`}
							data-tip={'No cookie 🤷‍♂️'}
						></div>
					{/if}
					<span class="italic">{enabled_env}:</span>
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
