<script lang="ts">
	import { availableProfilesStore } from '$lib/stores/available-profiles-store';
	import { execute } from '$lib/stores/error-store';
	import { featuresStore } from '$lib/stores/feature-store';
	import { AwsEnv, type RdsInstance } from '$lib/types';
	import { ask } from '@tauri-apps/api/dialog';

	export let database: RdsInstance;
	let {infraProfiles} = availableProfilesStore;
</script>

{#await $infraProfiles then infraProfiles}
	{#if $featuresStore.devWay || infraProfiles.some((profile) => profile == database.normalized_name)}
		<div class="tooltip tooltip-left" data-tip="Start proxy">
			<button
				class="flex flex-row gap-1"
				on:click={async () => {
					if (database?.env == AwsEnv.PROD) {
						let response = await ask(
							'Understand the risks before connecting to production database.\nUnauthorized or unintended changes can have severe consequences.\nProceed with care.',
							{
								title: 'Access to PRODUCTION database.',
								okLabel: 'Proceed',
								cancelLabel: 'Abort',
								type: 'warning'
							}
						);
						if (!response) {
							return;
						}
					}
					execute('start_db_proxy', { db: database });
				}}
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
							d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125"
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
	{:else}
		<div class="tooltip tooltip-left" data-tip={`Missing aws profile: ${database.normalized_name}`}>
			<button
				class="flex flex-row gap-1 opacity-30"
				disabled
				on:click={async () => {
					if (database?.env == AwsEnv.PROD) {
						let response = await ask(
							'Understand the risks before connecting to production database.\nUnauthorized or unintended changes can have severe consequences.\nProceed with care.',
							{
								title: 'Access to PRODUCTION database.',
								okLabel: 'Proceed',
								cancelLabel: 'Abort',
								type: 'warning'
							}
						);
						if (!response) {
							return;
						}
					}
					execute('start_db_proxy', { db: database });
				}}
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
							d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125"
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
	{/if}
{/await}
