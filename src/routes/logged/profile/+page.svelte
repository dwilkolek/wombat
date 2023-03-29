<script lang="ts">
	import { invoke } from '@tauri-apps/api/tauri';
	import { prevent_default } from 'svelte/internal';
	import type { UserConfig } from '../../types';

	let userConfigPromise = invoke<UserConfig>('user_config');
	let dbeaver_path: string | undefined = undefined;
	$: userConfigPromise.then((userConfig) => {
		dbeaver_path = userConfig?.dbeaver_path;
	});
</script>

<svelte:head>
	<title>Profile</title>
	<meta name="description" content="Wombat" />
</svelte:head>

{#await userConfigPromise then _}
	<form class="flex justify-center flex-col justify-items-center">
		<div class="form-control w-full min-w-xs">
			<label class="label" for="dbeaver_path">
				<span class="label-text">Path to dbeaver</span>
			</label>
			<input
				id="dbeaver_path"
				type="text"
				placeholder="DB path"
				class="input input-bordered w-full min-w-xs w-full"
				bind:value={dbeaver_path}
			/>

			<button
				on:click|preventDefault={() => {
					invoke('set_dbeaver_path', { dbeaverPath: dbeaver_path });
				}}>Save</button
			>
			<button
				on:click|preventDefault={() => {
					invoke('open_dbeaver');
				}}>TRIGGER</button
			>
		</div>
	</form>
{/await}
