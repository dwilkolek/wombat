<script lang="ts">
	import { proxyAuthConfigsStore } from '$lib/stores/proxy-auth-configs-store';
	import {
		AwsEnv,
		type ProxyAuthConfig,
		type ServiceDetails,
		type InfraProfile,
		type SsoProfile
	} from '$lib/types';
	import { ask, message } from '@tauri-apps/api/dialog';
	import { featuresStore } from '$lib/stores/feature-store';
	import { wombatProfileStore } from '$lib/stores/available-profiles-store';
	import CustomHeaderForm from './custom-header-form.svelte';
	import { type CustomHeader } from '$lib/types';
	import { taskStore, type NewTaskParams, TaskStatus } from '$lib/stores/task-store';
	import { invoke } from '@tauri-apps/api/tauri';

	export let service: ServiceDetails;
	let dialog: HTMLDialogElement;
	let selectedInfraProfile =
		$wombatProfileStore.infraProfiles.find(
			(infraProfile) => infraProfile.env == service.env && infraProfile.app == service.name
		) ?? $wombatProfileStore.infraProfiles.at(0);
	let selectedSsoProxy =
		$wombatProfileStore.ssoProfiles.find((ssoProfile) => ssoProfile.env == service.env) ??
		$wombatProfileStore.ssoProfiles.at(0);
	let useDevWayFeature = false;
	let selectedAuthInterceptor: ProxyAuthConfig | undefined;
	let customHeaders: CustomHeader[] = [
		{
			name: 'Host',
			encodeBase64: false,
			value: `${service.name}.service`
		}
	];

	$: matchingInfraProfiles =
		$wombatProfileStore.infraProfiles.filter((infraProfile) => infraProfile.env == service.env) ??
		[];
	$: isStartButtonDisabled =
		matchingInfraProfiles.length === 0 ||
		$taskStore.some((t) => t.arn == service.arn && t.status == TaskStatus.STARTING);

	$: proxyAuthConfigsForThisService = $proxyAuthConfigsStore.filter(
		(config) => config.env == service.env && config.toApp == service.name
	);

	$: filterForInfraProfile = (
		configs: ProxyAuthConfig[],
		infraProfile: InfraProfile | undefined
	) => {
		return configs.filter(
			(config) =>
				useDevWayFeature ||
				(infraProfile && (config.fromApp == '*' || infraProfile.app == config.fromApp))
		);
	};

	const startProxy = async (
		infraProfile: InfraProfile | undefined,
		ssoProfile: SsoProfile | undefined,
		proxyAuthConfig: ProxyAuthConfig | undefined,
		customHeadersList: CustomHeader[]
	) => {
		if (service?.env == AwsEnv.PROD) {
			let response = await ask(
				'Understand the risks before connecting to production service.\nUnauthorized or unintended changes can have severe consequences.\nProceed with care.',
				{
					title: 'Access to PRODUCTION service.',
					okLabel: 'Proceed',
					cancelLabel: 'Abort',
					type: 'warning'
				}
			);
			if (!response) {
				return;
			}
		}
		const headers: { [key: string]: string } = {};
		customHeadersList.forEach((header) => {
			headers[header.name] = header.encodeBase64 ? btoa(header.value) : header.value;
		});
		taskStore.startTask({ ...service, proxyAuthConfig }, async () => {
			return invoke<NewTaskParams>('start_service_proxy', {
				service,
				proxyAuthConfig,
				infraProfile,
				ssoProfile,
				headers
			});
		});
		dialog.close();
	};
</script>

<div
	class="tooltip tooltip-left h-[20px]"
	data-tip={isStartButtonDisabled ? 'Missing role allowing to setup proxy' : 'Start proxy'}
>
	<button
		disabled={isStartButtonDisabled}
		class={`flex flex-row gap-1 items-center cursor-pointer ${isStartButtonDisabled ? 'opacity-30' : ''}`}
		on:click={() => dialog.show()}
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
				class="w-3 h-3 absolute text-xs right-0 bottom-0 text-accent"
			>
				<path
					d="M6.3 2.841A1.5 1.5 0 004 4.11V15.89a1.5 1.5 0 002.3 1.269l9.344-5.89a1.5 1.5 0 000-2.538L6.3 2.84z"
				/>
			</svg>
		</div>
	</button>
</div>
<dialog
	bind:this={dialog}
	on:close={() => console.log('closed')}
	class="modal bg-black bg-opacity-60"
>
	<div class="modal-box w-11/12 max-w-[960px]">
		<div class="flex flex-col gap-4">
			<div class="flex flex-col gap-2">
				<div class="flex gap-2 items-center justify-between">
					<h2 class="flex items-center gap-2">
						<span class="text-lg font-h2 font-bold">Setup proxy:</span>
						<span>App: <b>{service.name}</b></span> | <span>Env: <b>{service.env}</b></span>
					</h2>
					<button
						class="btn btn-circle btn-sm"
						on:click|preventDefault={() => {
							dialog.close();
						}}
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
			<div class="flex flex-col gap-1">
				<span>Profile:</span>
				<div class="flex gap-4 items-end">
					<div class="w-32">
						<!-- svelte-ignore a11y-autofocus -->
						<select
							autofocus
							class="select select-bordered w-full select-sm"
							bind:value={useDevWayFeature}
						>
							<option value={false}> Infra </option>
							<option value={true} disabled={!$featuresStore.devWay}> SSO </option>
						</select>
					</div>
					{#if useDevWayFeature}
						<div class="grow">
							<!-- svelte-ignore a11y-autofocus -->
							<select
								autofocus
								class="select select-bordered w-full select-sm"
								bind:value={selectedSsoProxy}
							>
								{#each $wombatProfileStore.ssoProfiles as ssoProfile}
									{#if ssoProfile.env == service.env}{@const interceptorCount =
											filterForInfraProfile(proxyAuthConfigsForThisService, undefined).length}
										<option value={ssoProfile}>
											{ssoProfile.profile_name} - {interceptorCount} interceptor(s)
										</option>
									{/if}
								{/each}
							</select>
						</div>
					{/if}
					{#if !useDevWayFeature}
						<div class="grow">
							<!-- svelte-ignore a11y-autofocus -->
							<select
								autofocus
								class="select select-bordered w-full select-sm"
								bind:value={selectedInfraProfile}
							>
								{#each matchingInfraProfiles as infraProfile}
									{@const interceptorCount = filterForInfraProfile(
										proxyAuthConfigsForThisService,
										infraProfile
									).length}
									<option value={infraProfile}>
										{infraProfile.profile_name} - {interceptorCount} interceptor(s)
									</option>
								{/each}
							</select>
						</div>
					{/if}
				</div>
			</div>
			<div class="flex flex-col gap-1">
				<span>Authentication interceptor:</span>
				<select
					class="select select-bordered w-full select-sm"
					bind:value={selectedAuthInterceptor}
				>
					<option value={undefined}>None</option>

					{#each filterForInfraProfile(proxyAuthConfigsForThisService, selectedInfraProfile) as config}
						<option value={config}>
							{config.authType}: {config.jepsenClientId ?? config.basicUser ?? '?'}
						</option>
					{/each}
				</select>
			</div>

			<div>
				<div class="flex items-center gap-2 pb-2">
					Headers <button
						class="btn btn-xs btn-accent"
						disabled={!$featuresStore.proxyCustomHeaders}
						on:click={() => {
							let uuid = 'a77e0899-bb86-4551-b737-f28971f2d943';
							if (service.env == AwsEnv.DEMO) {
								uuid = '0a8d41aa-f38d-45fc-852b-6a01f57bbc54';
							}
							if (service.env == AwsEnv.PROD) {
								uuid = 'b0152a54-650e-47eb-87e0-075776ab3860';
							}
							customHeaders = [
								...customHeaders.filter(
									(h) => !['USER-UUID', 'USER-EMAIL', 'USER-NAME', 'USER-ROLES'].includes(h.name)
								),
								{
									name: 'USER-UUID',
									value: uuid,
									encodeBase64: true
								},
								{
									name: 'USER-EMAIL',
									value: 'Johnny.Oil@outlook.com',
									encodeBase64: true
								},
								{
									name: 'USER-NAME',
									value: 'Johnny Oil',
									encodeBase64: true
								},
								{
									name: 'USER-ROLES',
									value: 'ADMIN,USER',
									encodeBase64: true
								}
							];
						}}>+ example user</button
					>

					<button
						class="btn btn-xs btn-accent"
						disabled={!$featuresStore.proxyCustomHeaders}
						on:click={() => {
							customHeaders = [
								{
									name: 'Host',
									encodeBase64: false,
									value: `${service.name}.service`
								},
								...customHeaders.filter((h) => h.name.toLowerCase() !== 'host')
							];
						}}>+ host</button
					>
				</div>
				<div class="flex gap-1 flex-col">
					{#each customHeaders as header}
						{#key JSON.stringify(header)}
							<CustomHeaderForm
								added={true}
								{header}
								disabled={!$featuresStore.proxyCustomHeaders}
								onRemove={(name) => {
									customHeaders = [...customHeaders].filter((ch) => ch.name !== name);
								}}
							/>
						{/key}
					{/each}
					{#if $featuresStore.proxyCustomHeaders}
						<hr class="h-px my-1 bg-gray-200 border-0 dark:bg-gray-700" />
						<CustomHeaderForm
							added={false}
							disabled={!$featuresStore.proxyCustomHeaders}
							header={{ encodeBase64: false, name: '', value: '' }}
							onAdd={(header) => {
								if (
									customHeaders.some((ch) => header.name.toLowerCase() == ch.name.toLowerCase())
								) {
									message(`Header name needs to be unique`, { title: 'Ooops!', type: 'error' });
									throw Error('invalid header');
								}
								customHeaders = [...customHeaders, header];
							}}
						/>
					{/if}
				</div>
			</div>

			<div class="flex flex-row justify-end gap-2 mt-2">
				<button
					disabled={!selectedInfraProfile && !selectedSsoProxy}
					class="btn btn-active btn-accent btn-sm"
					on:click|preventDefault={() => {
						startProxy(
							useDevWayFeature ? undefined : selectedInfraProfile,
							useDevWayFeature ? selectedSsoProxy : undefined,
							selectedAuthInterceptor,
							customHeaders
						);
					}}
				>
					Start proxy</button
				>
			</div>
		</div>
	</div>
</dialog>
