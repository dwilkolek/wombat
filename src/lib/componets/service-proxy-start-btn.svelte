<script lang="ts">
	import { proxyAuthConfigsStore } from '$lib/stores/proxy-auth-configs-store';
	import { AwsEnv, type ProxyAuthConfig, type ServiceDetails } from '$lib/types';
	import { invoke } from '@tauri-apps/api';
	import { ask } from '@tauri-apps/api/dialog';
	import { availableProfilesStore } from '$lib/stores/available-profiles-store';
	import { featuresStore } from '$lib/stores/feature-store';

	export let service: ServiceDetails;
	let { infraProfiles } = availableProfilesStore;

	const start_proxy = async (proxyAuthConfig: ProxyAuthConfig | null) => {
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
		invoke('start_service_proxy', { service, proxyAuthConfig });
	};
</script>

{#if $featuresStore.devWay || $infraProfiles.some(([app, env]) => app == service.name && env == service.env)}
	<div class="tooltip tooltip-left h-[20px]" data-tip="Start proxy">
		<div class="dropdown">
			<div tabindex="0" role="button" class="flex flex-row gap-1 items-center cursor-pointer">
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
			</div>

			<!-- svelte-ignore a11y-no-noninteractive-tabindex -->
			<ul tabindex="0" class="shadow dropdown-content z-[1] menu bg-base-100 rounded-box w-52">
				<li>
					<button on:click|preventDefault={() => start_proxy(null)}>No auth proxy</button>
				</li>
				{#each $proxyAuthConfigsStore as config}
					{@const disabled =
						!$featuresStore.devWay &&
						!$infraProfiles.some(
							([app, env]) => env == service.env && (app == config.fromApp || config.fromApp == '*')
						)}

					{#if config.toApp == service.name && config.env == service.env}
						<li class={disabled ? 'opacity-30 cursor-not-allowed' : ''}>
							<button
								{disabled}
								on:click|preventDefault={() => {
									console.log('click');
									start_proxy(config);
								}}
							>
								{config.authType}: {config.jepsenClientId ?? config.basicUser ?? '?'}</button
							>
						</li>
					{/if}
				{/each}
			</ul>
		</div>
	</div>
{:else}
	<div class="tooltip tooltip-left h-[20px]" data-tip={`Missing aws profile: ${service.name}`}>
		<div class="flex flex-row gap-1 items-center">
			<div class="w-5 h-5 relative opacity-30">
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
		</div>
	</div>
{/if}
