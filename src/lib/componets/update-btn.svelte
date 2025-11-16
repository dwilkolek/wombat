<script lang="ts">
	import { check } from '@tauri-apps/plugin-updater';
	import { relaunch } from '@tauri-apps/plugin-process';
	import { UpdateButtonState } from '$lib/types';

	let btnState: UpdateButtonState = $state(UpdateButtonState.CHECK_DONE);
	let contentSize = $state(0);
	let downloadedSize = $state(0);
	let installProgress = $state(0);
	let errorMessage = $state('');
</script>

<div class="flex items-center fixed bottom-0 right-0 p-2 rounded-tl-md text-xs">
	{#await check()}
		<span>Checking for updates</span>
	{:then update}
		{#if update == null}
			<span>🎉 You're up to date, thanks!</span>
		{:else}
			<button
				class="btn btn-primary btn-sm"
				disabled={btnState != UpdateButtonState.CHECK_DONE}
				onclick={async () => {
					try {
						btnState = UpdateButtonState.DOWNLOADING;
						await update.downloadAndInstall((e) => {
							if (e.event == 'Started') {
								contentSize = e.data.contentLength ?? 0;
							} else if (e.event == 'Progress') {
								downloadedSize += e.data.chunkLength ?? 0;
							}
							installProgress = Math.round((downloadedSize / contentSize) * 100);
							if (contentSize == downloadedSize) {
								btnState = UpdateButtonState.INSTALLING;
							}
						});

						btnState = UpdateButtonState.INSTALLED;
						setTimeout(relaunch, 1500);
					} catch (e) {
						errorMessage = JSON.stringify(e);
						btnState = UpdateButtonState.FAILED;
					}
				}}
			>
				{#if btnState == UpdateButtonState.CHECK_DONE}
					<svg
						xmlns="http://www.w3.org/2000/svg"
						fill="none"
						viewBox="0 0 24 24"
						stroke-width="1.5"
						stroke="currentColor"
						class="size-6"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							d="M15.362 5.214A8.252 8.252 0 0 1 12 21 8.25 8.25 0 0 1 6.038 7.047 8.287 8.287 0 0 0 9 9.601a8.983 8.983 0 0 1 3.361-6.867 8.21 8.21 0 0 0 3 2.48Z"
						/>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							d="M12 18a3.75 3.75 0 0 0 .495-7.468 5.99 5.99 0 0 0-1.925 3.547 5.975 5.975 0 0 1-2.133-1.001A3.75 3.75 0 0 0 12 18Z"
						/>
					</svg>
					Update to {update.version}
				{:else if btnState == UpdateButtonState.DOWNLOADING}
					<span class="loading loading-spinner"></span> Downloading {installProgress}%
				{:else if btnState == UpdateButtonState.INSTALLING}
					<span class="loading loading-spinner"></span> Installing...
				{:else if btnState == UpdateButtonState.INSTALLED}
					<span class="loading loading-spinner"></span> Restarting...
				{:else if btnState == UpdateButtonState.FAILED}
					{errorMessage}
				{/if}
			</button>
		{/if}
	{/await}
</div>
