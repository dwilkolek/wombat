<script lang="ts">
	import { execute } from '$lib/error-store';
	import { taskStore } from '$lib/task-store';
	import type { AwsEnv, ServiceDetails } from '$lib/types';
	import { ask } from '@tauri-apps/api/dialog';
	import { open } from '@tauri-apps/api/shell';

	export let service: ServiceDetails;
	export let port: number;
</script>

<div class="tooltip" data-tip="Stop proxy to service">
	<button
		on:click={async () => {
			await execute('stop_job', { arn: service?.arn });
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
					d="M5.75 3a.75.75 0 00-.75.75v12.5c0 .414.336.75.75.75h1.5a.75.75 0 00.75-.75V3.75A.75.75 0 007.25 3h-1.5zM12.75 3a.75.75 0 00-.75.75v12.5c0 .414.336.75.75.75h1.5a.75.75 0 00.75-.75V3.75a.75.75 0 00-.75-.75h-1.5z"
				/>
			</svg>
		</div>
	</button>
	<div class="tooltip" data-tip="Open in browser">
		<button
			class="link"
			on:click|preventDefault={() => {
				open('http://localhost:' + port);
			}}
		>
			{service.version} @ :{$taskStore.find((t) => t.arn == service?.arn)?.port}</button
		>
	</div>
</div>
