<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import type { ServiceDetails } from '$lib/types';
	import { userStore } from '$lib/stores/user-store';
	import { restartEcsDisabledReason } from '$lib/stores/reasons';
	import DeploymentStatus from './deployment-status.svelte';

	interface Props {
		service: ServiceDetails;
	}

	let { service }: Props = $props();

	let disabledReason = restartEcsDisabledReason(service);
</script>

{#if $disabledReason?.deployment != null}
	{@const deployment = $disabledReason.deployment}
	<DeploymentStatus {deployment} />
{:else}
	<span
		class="tooltip tooltip-left flex"
		data-tip={$disabledReason?.message ?? 'Restart ECS Service'}
	>
		<button
			aria-label="Restart ECS Service"
			data-umami-event="ecs_task_restart_start"
			data-umami-event-uid={$userStore.id}
			disabled={!!$disabledReason}
			class={$disabledReason ? 'opacity-30' : ''}
			onclick={(e) => {
				e.preventDefault();
				invoke('deploy_ecs_service', {
					clusterArn: service.cluster_arn,
					serviceArn: service.arn,
					desiredVersion: null,
					includeTerraformTag: false
				});
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
	</span>
{/if}
