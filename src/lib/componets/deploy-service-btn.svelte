<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import { deplyomentStore } from '$lib/stores/deployment-store';
	import type { ServiceDetails } from '$lib/types';
	import { userStore } from '$lib/stores/user-store';
	import { restartEcsDisabledReason, deployEcsServiceDisabledReason } from '$lib/stores/reasons';

	interface Props {
		service: ServiceDetails;
	}

	let { service }: Props = $props();

	let disabledRestartReason = restartEcsDisabledReason(service);
	let disabledDeployNewVersionReason = deployEcsServiceDisabledReason(service);

	let disabledReason = $derived($disabledRestartReason || $disabledDeployNewVersionReason);
	let dialog: HTMLDialogElement | undefined = $state();
	let imageTag = $state(service.version);
	let justRestart = $state(true);
	let deployStarting = $state(false);
	let isValid = $derived(!justRestart ? imageTag.length > 3 : true);

	let command = $derived(justRestart ? 'ecs_task_restart_start' : 'ecs_task_deploy_start');
</script>

<span
	class="tooltip tooltip-left flex"
	data-tip={disabledReason?.message ??
		($disabledDeployNewVersionReason ? 'Deploy ECS Service' : 'Restart ECS Service')}
>
	{#if disabledReason?.deployment != null}
		{@const deployment = disabledReason.deployment}
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
				onclick={() => deployment && deplyomentStore.clear(deployment.deployment_id)}
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
		{#if deployment.rollout_status == 'Failed'}
			<button
				aria-label="Clear ECS restart state"
				class="text-rose-700"
				onclick={() => deployment && deplyomentStore.clear(deployment.deployment_id)}
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
				onclick={() => deployment && deplyomentStore.clear(deployment.deployment_id)}
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
	{:else if $disabledDeployNewVersionReason}
		<button
			aria-label="Restart ECS Service"
			data-umami-event={command}
			data-umami-event-uid={$userStore.id}
			disabled={!!disabledReason}
			class={disabledReason ? 'opacity-30' : ''}
			onclick={(e) => {
				e.preventDefault();

				invoke('deploy_ecs_service', {
					clusterArn: service.cluster_arn,
					serviceArn: service.arn,
					desiredVersion: null
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
	{:else}
		<button
			aria-label="Deploy ECS Service"
			disabled={!!disabledReason}
			class={disabledReason ? 'opacity-30' : ''}
			onclick={(e) => {
				e.preventDefault();
				dialog?.show();
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
					d="M3 16.5v2.25A2.25 2.25 0 0 0 5.25 21h13.5A2.25 2.25 0 0 0 21 18.75V16.5m-13.5-9L12 3m0 0 4.5 4.5M12 3v13.5"
				/>
			</svg>
		</button>
	{/if}
</span>

<dialog
	bind:this={dialog}
	onclose={() => console.log('closed')}
	class="modal bg-black bg-opacity-60"
>
	<div class="modal-box w-11/12 max-w-[960px]">
		<div class="flex flex-col gap-4">
			<div class="flex flex-col gap-2">
				<div class="flex gap-2 items-center justify-between">
					<h2 class="flex items-center gap-2">Deploy ECS service</h2>
					<button
						class="btn btn-circle btn-sm"
						onclick={(e) => {
							e.preventDefault();
							dialog?.close();
						}}
						aria-label="Close dialog"
					>
						<svg
							xmlns="http://www.w3.org/2000/svg"
							class="h-6 w-6"
							fill="none"
							viewBox="0 0 24 24"
							stroke="currentColor"
							><path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="2"
								d="M6 18L18 6M6 6l12 12"
							/></svg
						>
					</button>
				</div>
			</div>

			<div class="flex flex-row gap-4 mt-2">
				<input
					disabled={!!$disabledDeployNewVersionReason || justRestart}
					autocomplete="off"
					autocorrect="off"
					autocapitalize="off"
					spellcheck="false"
					type="text"
					placeholder="Image tag"
					bind:value={imageTag}
					class="input input-sm input-bordered grow"
				/>

				<div class="flex items-center gap-1">
					<input type="checkbox" class="toggle" bind:checked={justRestart} />
					Just restart
				</div>
				<button
					data-umami-event={command}
					data-umami-event-uid={$userStore.id}
					class="btn btn-active btn-accent btn-sm"
					disabled={!isValid || deployStarting}
					onclick={async (e) => {
						e.preventDefault();
						deployStarting = true;
						await invoke('deploy_ecs_service', {
							clusterArn: service.cluster_arn,
							serviceArn: service.arn,
							desiredVersion: justRestart ? null : imageTag
						});
						dialog?.close();
					}}
				>
					Run deployment</button
				>
			</div>
		</div>
	</div>
</dialog>
