<script lang="ts">
	import { proxyAuthConfigsStore } from '$lib/stores/proxy-auth-configs-store';
	import {
		AwsEnv,
		type ProxyAuthConfig,
		type InfraProfile,
		type SsoProfile,
		type EcsService
	} from '$lib/types';
	import { message } from '@tauri-apps/plugin-dialog';
	import { featuresStore } from '$lib/stores/feature-store';
	import { wombatProfileStore } from '$lib/stores/available-profiles-store';
	import CustomHeaderForm from './custom-header-form.svelte';
	import { type CustomHeader } from '$lib/types';
	import { taskStore, type NewTaskParams } from '$lib/stores/task-store';
	import { invoke } from '@tauri-apps/api/core';
	import { userStore } from '$lib/stores/user-store';
	import { startEcsProxyDisabledReason } from '$lib/stores/reasons';
	import { getFromList } from '$lib/utils';

	interface Props {
		service: EcsService;
	}

	let { service }: Props = $props();
	let dialog: HTMLDialogElement | undefined = $state();
	let selectedInfraProfile = $state(
		$wombatProfileStore.infraProfiles.find(
			(infraProfile) => infraProfile.env == service.env && infraProfile.app == service.name
		) ?? $wombatProfileStore.infraProfiles.at(0)
	);
	let selectedSsoProxy = $state(
		$wombatProfileStore.ssoProfiles.find((ssoProfile) => ssoProfile.env == service.env) ??
			$wombatProfileStore.ssoProfiles.at(0)
	);
	let useSSOProfile = $state(false);
	let selectedAuthInterceptor: ProxyAuthConfig | undefined = $state();
	const baseAddress = `https://${service.name}${service.env.toLowerCase() == 'prod' ? '' : '.' + service.env.toLowerCase()}.services.technipfmc.com`;
	let customHeaders: CustomHeader[] = $state([
		{
			name: 'Host',
			encodeBase64: false,
			value: `${service.name}.service`
		},
		{
			name: 'Origin',
			encodeBase64: false,
			value: baseAddress + '/'
		}
		// {
		// 	name: 'Referer',
		// 	encodeBase64: false,
		// 	value: baseAddress
		// }
	]);

	let matchingInfraProfiles = $derived(
		$wombatProfileStore.infraProfiles.filter((infraProfile) => infraProfile.env == service.env) ??
			[]
	);

	let disabledReason = startEcsProxyDisabledReason(service);

	let proxyAuthConfigsForThisService = $derived(
		$proxyAuthConfigsStore.filter(
			(config) => config.env == service.env && config.toApp == service.name
		)
	);

	let filterForInfraProfile = (
		configs: ProxyAuthConfig[],
		infraProfile: InfraProfile | undefined
	) => {
		return configs.filter(
			(config) =>
				infraProfile &&
				(config.fromApp == '*' || infraProfile.app == config.fromApp) &&
				!config.requireSsoProfile
		);
	};

	let filterForSsoProfile = $derived((configs: ProxyAuthConfig[]) => {
		return configs;
	});

	let configsForProfile = $derived(
		useSSOProfile
			? filterForSsoProfile(proxyAuthConfigsForThisService)
			: filterForInfraProfile(proxyAuthConfigsForThisService, selectedInfraProfile)
	);

	const startProxy = async (
		infraProfile: InfraProfile | undefined,
		ssoProfile: SsoProfile | undefined,
		proxyAuthConfig: ProxyAuthConfig | undefined,
		customHeadersList: CustomHeader[]
	) => {
		const headers: { [key: string]: string } = {};
		customHeadersList.forEach((header) => {
			headers[header.name] = header.encodeBase64 ? btoa(header.value) : header.value;
		});
		taskStore.startTask({ ...service, proxyAuthConfig }, async () => {
			console.log('headers', headers);
			return invoke<NewTaskParams>('start_service_proxy', {
				service,
				proxyAuthConfig,
				infraProfile,
				ssoProfile,
				headers
			});
		});
		dialog?.close();
	};
</script>

<div class="tooltip tooltip-left h-5" data-tip={$disabledReason ?? 'Start proxy'}>
	<button
		disabled={!!$disabledReason}
		class={`flex flex-row gap-1 items-center ${$disabledReason ? 'opacity-30' : 'cursor-pointer'}`}
		onclick={() => dialog?.show()}
		aria-label={$disabledReason ?? 'Start proxy'}
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
<dialog bind:this={dialog} class="modal">
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
						onclick={(e) => {
							e.preventDefault();
							dialog?.close();
						}}
						aria-label="Close modal"
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
				{#if service?.env === AwsEnv.PROD}
					<div role="alert" class="alert alert-warning mb-2">
						<svg
							xmlns="http://www.w3.org/2000/svg"
							class="h-6 w-6 shrink-0 stroke-current"
							fill="none"
							viewBox="0 0 24 24"
						>
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="2"
								d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
							/>
						</svg>
						<span
							>Understand the risks before connecting to production service. Unauthorized or
							unintended changes can have severe consequences. Proceed with care.</span
						>
					</div>
				{/if}
				<span>Profile:</span>
				<div class="flex gap-4 items-end">
					<div class="w-32">
						<!-- svelte-ignore a11y_autofocus -->
						<select autofocus class="select w-full select-sm" bind:value={useSSOProfile}>
							<option value={false}> Infra </option>
							<option value={true}> SSO </option>
						</select>
					</div>
					{#if useSSOProfile}
						<div class="grow">
							<!-- svelte-ignore a11y_autofocus -->
							<select autofocus class="select w-full select-sm" bind:value={selectedSsoProxy}>
								{#each $wombatProfileStore.ssoProfiles as ssoProfile (ssoProfile.profile_name)}
									{#if ssoProfile.env == service.env}{@const interceptorCount = filterForSsoProfile(
											proxyAuthConfigsForThisService
										).length}
										<option value={ssoProfile}>
											{ssoProfile.profile_name} - {interceptorCount} interceptor(s)
										</option>
									{/if}
								{/each}
							</select>
						</div>
					{/if}
					{#if !useSSOProfile}
						<div class="grow">
							<!-- svelte-ignore a11y_autofocus -->
							<select autofocus class="select w-full select-sm" bind:value={selectedInfraProfile}>
								{#each matchingInfraProfiles as infraProfile (infraProfile.profile_name)}
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
				<select class="select w-full select-sm" bind:value={selectedAuthInterceptor}>
					<option value={undefined}>None</option>

					{#each configsForProfile as config (config.id)}
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
						onclick={() => {
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
						onclick={() => {
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
					<button
						class="btn btn-xs btn-accent"
						disabled={!$featuresStore.proxyCustomHeaders}
						onclick={() => {
							customHeaders = [
								{
									name: 'Origin',
									encodeBase64: false,
									value: baseAddress + '/'
								},
								...customHeaders.filter((h) => h.name.toLowerCase() !== 'origin')
							];
						}}>+ origin</button
					>
					<button
						class="btn btn-xs btn-accent"
						disabled={!$featuresStore.proxyCustomHeaders}
						onclick={() => {
							customHeaders = [
								{
									name: 'Referer',
									encodeBase64: false,
									value: baseAddress
								},
								...customHeaders.filter((h) => h.name.toLowerCase() !== 'referer')
							];
						}}>+ referer</button
					>
				</div>
				<div class="flex gap-1 flex-col">
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
					{#if $featuresStore.proxyCustomHeaders}
						<hr class="h-px my-1 bg-gray-200 border-0 dark:bg-gray-700" />
						<CustomHeaderForm
							added={false}
							disabled={!$featuresStore.proxyCustomHeaders}
							onAdd={(header) => {
								if (
									customHeaders.some((ch) => header.name.toLowerCase() == ch.name.toLowerCase())
								) {
									message(`Header name needs to be unique`, { title: 'Ooops!', kind: 'error' });
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
					data-umami-event="ecs_proxy_start"
					data-umami-event-uid={$userStore.id}
					onclick={(e) => {
						e.preventDefault();
						startProxy(
							useSSOProfile ? undefined : selectedInfraProfile,
							useSSOProfile ? selectedSsoProxy : undefined,
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
