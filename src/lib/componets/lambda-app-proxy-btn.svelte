<script lang="ts">
	import { featuresStore } from '$lib/stores/feature-store';
	import { taskStore } from '$lib/stores/task-store';
	import type { AwsEnv, CustomHeader } from '$lib/types';
	import { invoke } from '@tauri-apps/api/core';
	import CustomHeaderForm from './custom-header-form.svelte';
	import { message } from '@tauri-apps/plugin-dialog';
	import { userStore } from '$lib/stores/user-store';
	import { wombatProfileStore } from '$lib/stores/available-profiles-store';
	import { startLambdaProxyDisabledReason } from '$lib/stores/reasons';
	import { getFromList, lambdaAppArn } from '$lib/utils';

	interface Props {
		app: string;
		env: AwsEnv;
	}

	let { app, env }: Props = $props();
	const lambdaArn = lambdaAppArn(app, env);
	let disabledReason = startLambdaProxyDisabledReason(lambdaArn, env);
	let port = $derived(
		$taskStore?.find((t) => {
			return t.arn === lambdaArn;
		})?.port
	);

	let dialog: HTMLDialogElement | undefined = $state();

	let availableApps = $derived(
		new Set([
			'none',
			'dxp',
			...$wombatProfileStore.infraProfiles
				.filter((infra) => infra.env == env)
				.map((infra) => infra.app)
		])
	);

	let selectedApp = $state('none');

	let defaultHeaders = $derived(getAppHeaders(selectedApp ?? 'none'));
	let customHeaders: CustomHeader[] = $state([]);

	function getAppHeaders(app: string) {
		if (app == 'none') {
			return [];
		}
		const baseAddress = `https://${app}${env.toLowerCase() == 'prod' ? '' : '.' + env.toLowerCase()}.services.technipfmc.com`;
		return [
			{
				name: 'Origin',
				encodeBase64: false,
				value: baseAddress + '/'
			},
			{
				name: 'Referer',
				encodeBase64: false,
				value: baseAddress
			}
		];
	}

	const startProxy = async () => {
		const headers: { [key: string]: string } = {};
		[...defaultHeaders, ...customHeaders].forEach((header) => {
			headers[header.name] = header.encodeBase64 ? btoa(header.value) : header.value;
		});
		taskStore.startTask(
			{
				arn: lambdaArn,
				name: app
			},
			async () =>
				invoke('start_lambda_app_proxy', {
					app,
					env,
					address: `https://${app}${env.toLowerCase() == 'prod' ? '' : '.' + env.toLowerCase()}.services.technipfmc.com/`,
					headers
				})
		);
		dialog?.close();
	};
</script>

{#if !port}
	<div class="tooltip tooltip-left h-5" data-tip={$disabledReason ?? 'Start proxy'}>
		<button
			aria-label="Start proxy"
			disabled={!!$disabledReason}
			class={`flex flex-row gap-1 items-center cursor-pointer ${$disabledReason ? 'opacity-30' : ''}`}
			onclick={() => dialog?.show()}
		>
			<div class="w-5 h-5 relative">
				<svg
					xmlns="http://www.w3.org/2000/svg"
					fill="none"
					viewBox="0 0 24 24"
					stroke-width="1.5"
					stroke="currentColor"
					class="w-4 h-4 absolute"
				>
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						d="M21.75 17.25v-.228a4.5 4.5 0 00-.12-1.03l-2.268-9.64a3.375 3.375 0 00-3.285-2.602H7.923a3.375 3.375 0 00-3.285 2.602l-2.268 9.64a4.5 4.5 0 00-.12 1.03v.228m19.5 0a3 3 0 01-3 3H5.25a3 3 0 01-3-3m19.5 0a3 3 0 00-3-3H5.25a3 3 0 00-3 3m16.5 0h.008v.008h-.008v-.008zm-3 0h.008v.008h-.008v-.008z"
					/>
				</svg>
				<svg
					xmlns="http://www.w3.org/2000/svg"
					viewBox="0 0 20 20"
					fill="currentColor"
					class="w-3 h-3 absolute text-xs right-0 bottom-0 text-success"
				>
					<path
						d="M6.3 2.841A1.5 1.5 0 004 4.11V15.89a1.5 1.5 0 002.3 1.269l9.344-5.89a1.5 1.5 0 000-2.538L6.3 2.84z"
					/>
				</svg>
			</div>
		</button>
	</div>
{:else}
	<div class="tooltip tooltip-left flex" data-tip="Stop proxy to service">
		<button
			aria-label="Stop proxy to service"
			onclick={async () => {
				await invoke('stop_job', { arn: lambdaArn });
			}}
		>
			<div class="w-5 h-5 relative">
				<svg
					xmlns="http://www.w3.org/2000/svg"
					fill="none"
					viewBox="0 0 24 24"
					stroke-width="1.5"
					stroke="currentColor"
					class="w-4 h-4 absolute text-info"
				>
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						d="M21.75 17.25v-.228a4.5 4.5 0 00-.12-1.03l-2.268-9.64a3.375 3.375 0 00-3.285-2.602H7.923a3.375 3.375 0 00-3.285 2.602l-2.268 9.64a4.5 4.5 0 00-.12 1.03v.228m19.5 0a3 3 0 01-3 3H5.25a3 3 0 01-3-3m19.5 0a3 3 0 00-3-3H5.25a3 3 0 00-3 3m16.5 0h.008v.008h-.008v-.008zm-3 0h.008v.008h-.008v-.008z"
					/>
				</svg>
				<svg
					xmlns="http://www.w3.org/2000/svg"
					viewBox="0 0 20 20"
					fill="currentColor"
					class="w-3 h-3 absolute text-xs right-0 bottom-0 text-success"
				>
					<path
						d="M5.75 3a.75.75 0 00-.75.75v12.5c0 .414.336.75.75.75h1.5a.75.75 0 00.75-.75V3.75A.75.75 0 007.25 3h-1.5zM12.75 3a.75.75 0 00-.75.75v12.5c0 .414.336.75.75.75h1.5a.75.75 0 00.75-.75V3.75a.75.75 0 00-.75-.75h-1.5z"
					/>
				</svg>
			</div>
		</button>
	</div>
{/if}

<dialog bind:this={dialog} onclose={() => console.log('closed')} class="modal">
	<div class="modal-box w-11/12 max-w-[960px]">
		<div class="flex flex-col gap-4">
			<div class="flex flex-col gap-2">
				<div class="flex gap-2 items-center justify-between">
					<h2 class="flex items-center gap-2">
						<span class="text-lg font-h2 font-bold">Setup proxy:</span>
						<span>App: <b>{app}</b></span> | <span>Env: <b>{env}</b></span>
					</h2>
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
			<div>
				<div class="flex items-center gap-2 pb-2">
					Headers <select class="select w-full select-sm" bind:value={selectedApp}>
						>
						{#each availableApps as app (app)}
							<option value={app}> {app} </option>
						{/each}
					</select>
				</div>

				<div class="flex gap-1 flex-col">
					{#each getFromList(defaultHeaders) as header (header)}
						<CustomHeaderForm
							added={true}
							disabled={true}
							bind:name={header.name}
							bind:value={header.value}
							bind:encodeBase64={header.encodeBase64}
							onRemove={() => {
								console.error('cannot remove');
							}}
						/>
					{/each}
					{#each getFromList(customHeaders) as header (header)}
						<CustomHeaderForm
							added={true}
							bind:name={header.name}
							bind:value={header.value}
							bind:encodeBase64={header.encodeBase64}
							disabled={!$featuresStore.proxyCustomHeaders}
							onRemove={(name) => {
								customHeaders = [...customHeaders].filter((ch) => ch.name !== name);
							}}
						/>
					{/each}
					<hr class="h-px my-1 bg-gray-200 border-0 dark:bg-gray-700" />
					<CustomHeaderForm
						added={false}
						disabled={!$featuresStore.proxyCustomHeaders}
						onAdd={(header) => {
							if (customHeaders.some((ch) => header.name.toLowerCase() == ch.name.toLowerCase())) {
								message(`Header name needs to be unique`, { title: 'Ooops!', kind: 'error' });
								throw Error('invalid header');
							}
							customHeaders = [...customHeaders, header];
						}}
					/>
				</div>
			</div>

			<div class="flex flex-row justify-end gap-2 mt-2">
				<button
					data-umami-event="lambda_app_proxy_start"
					data-umami-event-uid={$userStore.id}
					class="btn btn-active btn-accent btn-sm"
					onclick={(e) => {
						e.preventDefault();
						startProxy();
					}}
				>
					Start proxy</button
				>
			</div>
		</div>
	</div>
</dialog>
