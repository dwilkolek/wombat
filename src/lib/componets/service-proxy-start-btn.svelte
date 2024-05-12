<script lang="ts">
	import { proxyAuthConfigsStore } from '$lib/stores/proxy-auth-configs-store';
	import { AwsEnv, type ProxyAuthConfig, type ServiceDetails, type InfraProfile } from '$lib/types';
	import { invoke } from '@tauri-apps/api';
	import { ask } from '@tauri-apps/api/dialog';
	import { featuresStore } from '$lib/stores/feature-store';
	import { wombatProfileStore } from '$lib/stores/available-profiles-store';

	export let service: ServiceDetails;
	let dialog: HTMLDialogElement;
	let selected_proxy_infra_profile =
		$wombatProfileStore.infraProfiles.find(
			(infraProfile) => infraProfile.env == service.env && infraProfile.app == service.name
		) ?? $wombatProfileStore.infraProfiles.at(0);
	let selected_proxy_auth_interceptor: ProxyAuthConfig | undefined;

	const start_proxy = async (
		infraProfile: InfraProfile,
		proxyAuthConfig: ProxyAuthConfig | null | undefined
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
		invoke('start_service_proxy', { service, proxyAuthConfig, infraProfile });
	};
</script>

<div class="tooltip tooltip-left h-[20px]" data-tip="Start proxy">
	<button class="flex flex-row gap-1 items-center cursor-pointer" on:click={() => dialog.show()}>
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
	<div class="modal-box">
		<div class="flex flex-col gap-2">
			<div>
				Using profile:
				<select
					class="select select-bordered w-full select-sm"
					bind:value={selected_proxy_infra_profile}
				>
					{#each $wombatProfileStore.infraProfiles as infraProfile}
						{#if infraProfile.env == service.env}
							<option value={infraProfile}>
								{infraProfile.profile_name}
							</option>
						{/if}
					{/each}
				</select>
			</div>

			<div>
				Authentication interceptor:
				<select
					class="select select-bordered w-full select-sm"
					bind:value={selected_proxy_auth_interceptor}
				>
					<option value={undefined}>None</option>

					{#each $proxyAuthConfigsStore as config}
						{#if selected_proxy_infra_profile && config.env == service.env && config.toApp == service.name && (config.fromApp == '*' || selected_proxy_infra_profile.app == config.fromApp)}
							<option value={config}>
								{config.authType}: {config.jepsenClientId ?? config.basicUser ?? '?'}
							</option>
						{/if}
					{/each}
				</select>
			</div>

			<div class="flex flex-row justify-end gap-2 my-2">
				<button
					disabled={!selected_proxy_infra_profile}
					class="btn btn-active btn-accent btn-sm"
					on:click|preventDefault={() => {
						console.log('click');
						selected_proxy_infra_profile &&
							start_proxy(selected_proxy_infra_profile, selected_proxy_auth_interceptor);
					}}
				>
					Start proxy</button
				>

				<button
					class="btn btn-active justify-items-end btn-error btn-sm"
					on:click|preventDefault={() => {
						dialog.close();
					}}
				>
					Close</button
				>
			</div>
		</div>
	</div>
</dialog>
