<script lang="ts">
	import { execute } from '$lib/stores/error-store';
	import { taskStore } from '$lib/stores/task-store';
	import { AwsEnv, type ServiceDetails } from '$lib/types';
	import { ask } from '@tauri-apps/api/dialog';
	import { open } from '@tauri-apps/api/shell';

	export let service: ServiceDetails;
</script>

<div class="tooltip" data-tip="Start proxy">
	<button
		class="flex flex-row gap-1"
		on:click={async () => {
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
			execute('start_service_proxy', { service });
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
