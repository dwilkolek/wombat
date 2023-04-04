<script lang="ts">
	import { invoke } from '@tauri-apps/api/tauri';
	import { prevent_default } from 'svelte/internal';
	import type { UserConfig } from '$lib/types';
	import { userStore } from '$lib/user-store';

	let user = $userStore;
	let dbeaver_path = user?.dbeaver_path ?? '';
</script>

<svelte:head>
	<title>PROFILE</title>
	<meta name="description" content="Wombat" />
</svelte:head>
<div class="max-w-md mx-auto">
	<form class="flex flex-col justify-items-center gap-2">
		<div class="form-control w-full min-w-xs">
			<label class="label" for="dbeaver_path">
				<span class="label-text">Path to dbeaver</span>
			</label>
			<input
				id="dbeaver_path"
				type="text"
				placeholder="DB path"
				class="input input-bordered w-full min-w-xs"
				bind:value={dbeaver_path}
			/>
		</div>

		<button
			class="btn btn-primary"
			on:click|preventDefault={() => {
				userStore.setDbeaverPath(dbeaver_path);
			}}
			>Save
		</button>
	</form>
</div>
