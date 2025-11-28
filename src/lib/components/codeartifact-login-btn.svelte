<script lang="ts">
	import { userStore } from '$lib/stores/user-store';
	import { invoke } from '@tauri-apps/api/core';
	let disabled = $state(true);
	let loading = $state(true);
	let status: 'success' | 'failed' | 'none' = $state('none');
	$effect(() => {
		loading = true;
		invoke('codeartifact_login_check')
			.then(() => {
				disabled = false;
				console.log('Success');
			})
			.catch((e) => console.error('Error', e))
			.finally(() => (loading = false));
	});
</script>

<button
	class="btn btn-square relative"
	type="button"
	data-umami-event={disabled ? 'codeartifact_login_check' : 'codeartifact_login'}
	data-umami-event-uid={$userStore.id}
	onclick={() => {
		if (disabled) {
			invoke('codeartifact_login_check')
				.then(() => {
					disabled = false;
					console.log('Success');
				})
				.catch((e) => console.error('Error', e))
				.finally(() => (loading = false));
		} else {
			loading = true;
			status = 'none';
			invoke('codeartifact_login')
				.then(() => {
					status = 'success';
				})
				.catch(() => {
					status = 'failed';
				})
				.finally(() => {
					loading = false;
					setTimeout(() => (status = 'none'), 3000);
				});
		}
	}}
>
	{#if loading}
		<span class="loading loading-spinner"></span>
	{:else}
		<svg
			class="w-5 h-5 {disabled ? 'grayscale' : ''}"
			viewBox="0 0 80 80"
			version="1.1"
			xmlns="http://www.w3.org/2000/svg"
			xmlns:xlink="http://www.w3.org/1999/xlink"
		>
			<title>Login to dsi_artifactory </title>
			<defs>
				<linearGradient x1="0%" y1="100%" x2="100%" y2="0%" id="linearGradient-1">
					<stop stop-color="#2E27AD" offset="0%"></stop>
					<stop stop-color="#527FFF" offset="100%"></stop>
				</linearGradient>
			</defs>
			<g
				id="Icon-Architecture/64/Arch_AWS-CodeArtifact_64"
				stroke="none"
				stroke-width="1"
				fill="none"
				fill-rule="evenodd"
			>
				<g id="Rectangle" fill="url(#linearGradient-1)">
					<rect x="0" y="0" width="80" height="80"></rect>
				</g>
				<g
					id="Icon-Service/64/AWS-CodeArtifact_64"
					transform="translate(8.000000, 8.000000)"
					fill="#FFFFFF"
				>
					<path
						d="M28.848,38.128 L34.773,24.893 L36.599,25.711 L30.674,38.945 L28.848,38.128 Z M37.143,34.097 L40.702,31.501 L37.567,28.321 L38.991,26.917 L42.94,30.922 C43.144,31.129 43.248,31.416 43.225,31.706 C43.202,31.996 43.052,32.261 42.818,32.432 L38.323,35.712 L37.143,34.097 Z M28.304,28.407 L24.694,31.042 L27.838,34.222 L26.416,35.628 L22.454,31.623 C22.249,31.416 22.145,31.129 22.168,30.839 C22.192,30.549 22.34,30.283 22.576,30.112 L27.124,26.792 L28.304,28.407 Z M27.152,5.721 L7,17.636 L7,40.871 L5,40.871 L5,17.066 C5,16.712 5.187,16.385 5.492,16.205 L26.134,4 L27.152,5.721 Z M60,22.871 L60,46.935 C60,47.293 59.809,47.623 59.499,47.802 L38.873,59.678 L37.875,57.945 L58,46.357 L58,22.871 L60,22.871 Z M48,40.664 L32.92,49.239 L18,40.496 L18,23.172 L32.91,14.599 L48,23.347 L48,40.664 Z M50,41.246 L50,22.771 C50,22.414 49.811,22.085 49.502,21.905 L33.414,12.579 C33.104,12.399 32.723,12.399 32.414,12.577 L16.502,21.727 C16.192,21.905 16,22.236 16,22.594 L16,41.069 C16,41.424 16.189,41.752 16.494,41.932 L32.407,51.257 C32.563,51.348 32.738,51.394 32.912,51.394 C33.083,51.394 33.253,51.351 33.407,51.263 L49.494,42.115 C49.807,41.937 50,41.605 50,41.246 L50,41.246 Z"
						id="AWS-CodeArtifact_Icon_64_Squid"
					></path>
				</g>
			</g>
		</svg>
		{#if status == 'success'}
			<svg
				xmlns="http://www.w3.org/2000/svg"
				fill="none"
				viewBox="0 0 24 24"
				stroke-width="1.5"
				stroke="currentColor"
				class="w-4 h-4 absolute right-0 bottom-0 text-lime-500"
			>
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					d="M9 12.75 11.25 15 15 9.75M21 12a9 9 0 1 1-18 0 9 9 0 0 1 18 0Z"
				/>
			</svg>
		{/if}

		{#if status == 'failed'}
			<svg
				xmlns="http://www.w3.org/2000/svg"
				fill="none"
				viewBox="0 0 24 24"
				stroke-width="1.5"
				stroke="currentColor"
				class="w-4 h-4 absolute right-0 bottom-0 text-rose-500"
			>
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					d="M12 9v3.75m9-.75a9 9 0 1 1-18 0 9 9 0 0 1 18 0Zm-9 3.75h.008v.008H12v-.008Z"
				/>
			</svg>
		{/if}
	{/if}
</button>
