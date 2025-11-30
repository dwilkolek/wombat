<script lang="ts">
	import { invoke } from '@tauri-apps/api/core';
	import type { ServiceDetails } from '$lib/types';
	import { userStore } from '$lib/stores/user-store';
	import { deployEcsServiceDisabledReason } from '$lib/stores/reasons';
	import DeploymentStatus from './deployment-status.svelte';
	import { featuresStore } from '$lib/stores/feature-store';

	interface Props {
		service: ServiceDetails;
	}

	let { service }: Props = $props();

	let disabledReason = deployEcsServiceDisabledReason(service);

	let dialog: HTMLDialogElement | undefined = $state();
	let imageTag = $state(service.version);
	let includeTerraformTag = $state(false);
	let deployStarting = $state(false);
</script>

{#if $disabledReason?.deployment != null}
	{@const deployment = $disabledReason.deployment}
	<DeploymentStatus {deployment} />
{:else}
	<span
		class="tooltip tooltip-left flex"
		data-tip={$disabledReason?.message ?? 'Deploy ECS Service'}
	>
		<button
			aria-label="Deploy ECS Service"
			disabled={!!$disabledReason}
			class={$disabledReason ? 'opacity-30' : ''}
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
	</span>
{/if}

<dialog bind:this={dialog} class="modal">
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

			<div class="flex flex-col w-full gap-4 mt-2">
				<div class="flex flex-row gap-4 items-center">
					<input
						disabled={!!$disabledReason || deployStarting}
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
						<input
							type="checkbox"
							class="toggle"
							bind:checked={includeTerraformTag}
							disabled={deployStarting || !$featuresStore.deployEcsWithTags}
						/>
						Set Terraform tag
					</div>
				</div>
				<div class="flex flex-row gap-4 items-center justify-end">
					<button
						data-umami-event="ecs_task_deploy_start"
						data-umami-event-uid={$userStore.id}
						class="btn btn-active btn-accent btn-sm"
						disabled={imageTag.length <= 3 || deployStarting}
						onclick={async (e) => {
							e.preventDefault();
							deployStarting = true;
							await invoke('deploy_ecs_service', {
								clusterArn: service.cluster_arn,
								serviceArn: service.arn,
								desiredVersion: imageTag,
								includeTerraformTag: includeTerraformTag
							});
							dialog?.close();
						}}
					>
						Deploy {imageTag} {includeTerraformTag ? 'and set Terraform=true' : ''}</button
					>
					<button
						data-umami-event="ecs_task_restart_start"
						data-umami-event-uid={$userStore.id}
						class="btn btn-active btn-secondary btn-sm"
						disabled={deployStarting}
						onclick={async (e) => {
							e.preventDefault();
							deployStarting = true;
							await invoke('deploy_ecs_service', {
								clusterArn: service.cluster_arn,
								serviceArn: service.arn,
								desiredVersion: null,
								includeTerraformTag: false
							});
							dialog?.close();
						}}
					>
						Restart
					</button>
				</div>
			</div>
		</div>
	</div>
</dialog>
