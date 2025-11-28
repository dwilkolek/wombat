<script lang="ts">
	import { deplyomentStore, type DeploymentRolloutStatus } from '$lib/stores/deployment-store';
	import { userStore } from '$lib/stores/user-store';

	interface Props {
		deployment: DeploymentRolloutStatus;
	}

	let { deployment }: Props = $props();
</script>

{#if deployment != null}
	<span
		class="tooltip tooltip-left flex"
		data-tip={`${deployment.rollout_status} (${deployment.version ?? 'restart'})
		${deployment.error_message ?? ''}`}
	>
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
				aria-label="Clear ECS restart state"
				class="text-lime-500"
				onclick={() => deplyomentStore.clear(deployment.deployment_id)}
				data-umami-event="ecs_deploy_clear"
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
						d="M9 12.75 11.25 15 15 9.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z"
					/>
				</svg>
			</button>
		{/if}
		{#if deployment.rollout_status == 'Failed'}
			<button
				aria-label="Clear ECS restart state"
				class="text-rose-500"
				onclick={() => deplyomentStore.clear(deployment.deployment_id)}
				data-umami-event="ecs_deploy_clear"
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
		{#if deployment.rollout_status == 'Unknown'}
			<button
				aria-label="Clear ECS restart state"
				class="text-sky-400"
				onclick={() => deplyomentStore.clear(deployment.deployment_id)}
				data-umami-event="ecs_deploy_clear"
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
						d="M9.879 7.519c1.171-1.025 3.071-1.025 4.242 0 1.172 1.025 1.172 2.687 0 3.712-.203.179-.43.326-.67.442-.745.361-1.45.999-1.45 1.827v.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9 5.25h.008v.008H12v-.008Z"
					/>
				</svg>
			</button>
		{/if}
	</span>
{/if}
