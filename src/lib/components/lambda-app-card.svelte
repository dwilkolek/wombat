<script lang="ts">
	import { wombatAccountStore } from '$lib/stores/available-accounts-store';
	import { taskStore } from '$lib/stores/task-store';
	import { get } from 'svelte/store';
	import AppCardHr from './app-card-hr.svelte';
	import LambdaTaskStatus from './lambda-task-status.svelte';
	import LambdaAppProxyBtn from './lambda-app-proxy-btn.svelte';
	import { cookieJar } from '$lib/stores/cookie-jar-status';
	import { lambdaAppArn } from '$lib/utils';
	interface Props {
		app: string;
	}

	let { app }: Props = $props();

	let ssoProfileName = $derived.by(() => {
		// Try to find the role that owns the infra profile for this app in any environment
		const ssoByInfra = $wombatAccountStore.ssoProfiles.find((sso) =>
			sso.infra_profiles.some((i) => i.app === app)
		);
		if (ssoByInfra) return ssoByInfra.profile_name;

		// Fallback to searching tasks in taskStore for this app
		const tasks = get(taskStore);
		for (const env of $wombatAccountStore.environments) {
			const task = tasks.find((t) => t.arn === lambdaAppArn(app, env));
			if (task?.sso_profile) return task.sso_profile;
		}

		return undefined;
	});
</script>

<div class="px-2 py-1 shadow-2xl w-full flex rounded-lg bg-base-300">
	<div class="min-w-80 w-80 flex flex-col gap-0.5 justify-center py-1">
		<div class="flex flex-row gap-2 items-center text-md">
			<span class="inline text-base pl-1">
				{app}
			</span>
		</div>
		{#if ssoProfileName}
			<div class="pl-1">
				<span class="opacity-70 font-medium text-xs italic">{ssoProfileName}</span>
			</div>
		{/if}
	</div>

	<div
		class="grid w-full divide-x divide-base-100"
		style={`grid-template-columns: repeat(${$wombatAccountStore.environments.length ?? 1}, minmax(0, 1fr));`}
	>
		{#each $wombatAccountStore.environments as enabled_env (enabled_env)}
			{@const task = $taskStore.find((task) => task.arn == lambdaAppArn(app, enabled_env))}
			<div class="flex flex-col app-env-cell px-2">
				<div class="font-medium text-xs flex items-row gap-1 items-center">
					{#if $cookieJar.cookieHealth[enabled_env] == 'Ok'}
						<div
							class="bg-lime-400 h-2 w-2 rounded tooltip tooltip-top"
							data-tip="Cookie is fresh, <5min"
						></div>
					{/if}
					{#if $cookieJar.cookieHealth[enabled_env] == 'Stale'}
						<div
							class="bg-amber-300 h-2 w-2 rounded tooltip tooltip-top"
							data-tip="Cookie is stale, >5min"
						></div>
					{/if}
					{#if $cookieJar.cookieHealth[enabled_env] == 'Old'}
						<div
							class="bg-red-500 h-2 w-2 rounded tooltip tooltip-top"
							data-tip="Cookie is old, >10min"
						></div>
					{/if}
					{#if !$cookieJar.cookieHealth[enabled_env]}
						<div
							class="bg-gray-500 h-2 w-2 rounded tooltip tooltip-top"
							data-tip="No cookie 🤷‍"
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
