<script lang="ts">
	import { invoke } from '@tauri-apps/api';
	import { deplyomentStore } from '$lib/stores/deployment-store';
	import type { EcsService } from '$lib/types';
	import { userStore } from '$lib/stores/user-store';
	import { restartEcsDisabledReason } from '$lib/stores/reasons';

	export let service: EcsService;

	$: disabledReason = restartEcsDisabledReason(service);
</script>

<span class="tooltip tooltip-left flex" data-tip={$disabledReason?.message ?? 'Restart service'}>
	{#if $disabledReason?.deployment != null}
		{@const deployment = $disabledReason.deployment}
		{#if deployment.rollout_status == 'In Progress'}
			<span class="text-amber-300">
				<svg
					xmlns="http://www.w3.org/2000/svg"
					fill="none"
					viewBox="0 0 24 24"
					stroke-width="1.5"
					stroke="currentColor"
					class="w-4 h-4"
				>
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						d="M12 6v6h4.5m4.5 0a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z"
					/>
				</svg>
			</span>
		{/if}
		{#if deployment.rollout_status == 'Completed'}
			<button
				class="text-lime-500"
				on:click={() => deployment && deplyomentStore.clear(deployment.deployment_id)}
				data-umami-event="ecs_restart_clear"
				data-umami-event-uid={$userStore.id}
			>
				<svg
					xmlns="http://www.w3.org/2000/svg"
					fill="none"
					viewBox="0 0 24 24"
					stroke-width="1.5"
					stroke="currentColor"
					class="w-4 h-4"
				>
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						d="M9 12.75 11.25 15 15 9.75M21 12c0 1.268-.63 2.39-1.593 3.068a3.745 3.745 0 0 1-1.043 3.296 3.745 3.745 0 0 1-3.296 1.043A3.745 3.745 0 0 1 12 21c-1.268 0-2.39-.63-3.068-1.593a3.746 3.746 0 0 1-3.296-1.043 3.745 3.745 0 0 1-1.043-3.296A3.745 3.745 0 0 1 3 12c0-1.268.63-2.39 1.593-3.068a3.745 3.745 0 0 1 1.043-3.296 3.746 3.746 0 0 1 3.296-1.043A3.746 3.746 0 0 1 12 3c1.268 0 2.39.63 3.068 1.593a3.746 3.746 0 0 1 3.296 1.043 3.746 3.746 0 0 1 1.043 3.296A3.745 3.745 0 0 1 21 12Z"
					/>
				</svg>
			</button>
		{/if}
		{#if deployment.rollout_status == 'Failed'}
			<button
				class="text-rose-700"
				on:click={() => deployment && deplyomentStore.clear(deployment.deployment_id)}
				data-umami-event="ecs_restart_clear"
				data-umami-event-uid={$userStore.id}
				><svg
					xmlns="http://www.w3.org/2000/svg"
					fill="none"
					viewBox="0 0 24 24"
					stroke-width="1.5"
					stroke="currentColor"
					class="w-4 h-4"
				>
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						d="M12 9v3.75m9-.75a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9 3.75h.008v.008H12v-.008Z"
					/>
				</svg>
			</button>
		{/if}
	{:else}
		<button
			data-umami-event="ecs_task_restart_start"
			data-umami-event-uid={$userStore.id}
			disabled={!!$disabledReason}
			class={$disabledReason ? 'opacity-30' : ''}
			on:click|preventDefault={(e) => {
				invoke('restart_service', {
					env: service.env,
					clusterArn: service.cluster_arn,
					serviceName: service.name
				});
				e.currentTarget.disabled = true;
			}}
		>
			<svg
				xmlns="http://www.w3.org/2000/svg"
				fill="none"
				viewBox="0 0 24 24"
				stroke-width="1.5"
				stroke="currentColor"
				class="w-4 h-4"
			>
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0 3.181 3.183a8.25 8.25 0 0 0 13.803-3.7M4.031 9.865a8.25 8.25 0 0 1 13.803-3.7l3.181 3.182m0-4.991v4.99"
				/>
			</svg>
		</button>
	{/if}
</span>
