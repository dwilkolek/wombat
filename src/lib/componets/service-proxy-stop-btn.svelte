<script lang="ts">
	import { userStore } from '$lib/stores/user-store';
	import { invoke } from '@tauri-apps/api/core';

	export let service_arn: string;
</script>

<div class="tooltip tooltip-left flex" data-tip="Stop proxy to service">
	<button
		on:click={async () => {
			await invoke('stop_job', { arn: service_arn });
		}}
		data-umami-event="ecs_proxy_stop"
		data-umami-event-uid={$userStore.id}
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
				class="w-3 h-3 absolute text-xs right-0 bottom-0 text-accent"
			>
				<path
					d="M5.75 3a.75.75 0 00-.75.75v12.5c0 .414.336.75.75.75h1.5a.75.75 0 00.75-.75V3.75A.75.75 0 007.25 3h-1.5zM12.75 3a.75.75 0 00-.75.75v12.5c0 .414.336.75.75.75h1.5a.75.75 0 00.75-.75V3.75a.75.75 0 00-.75-.75h-1.5z"
				/>
			</svg>
		</div>
	</button>
</div>
