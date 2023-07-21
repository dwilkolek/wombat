<script lang="ts">
	import { userStore } from '$lib/stores/user-store';
	import { open } from '@tauri-apps/api/shell';

	let user = $userStore;
	let dbeaver_path = user?.dbeaver_path ?? '';
</script>

<svelte:head>
	<title>CONFIG</title>
	<meta name="description" content="Wombat" />
</svelte:head>
<div class="mx-auto">
	<form
		class="flex flex-col justify-items-center gap-2"
		on:submit|preventDefault={() => {
			userStore.setDbeaverPath(dbeaver_path);
		}}
	>
		<div class="form-control">
			<div class="text-sm rounded p-4">
				<div class="flex flex-row gap-2 mb-4">
					<span class="text-lg">User Id:</span>
					<span class="text-lg">{user.id}</span>
				</div>

				<label class="label p-0 m-0" for="dbeaver_path">
					<span class="label-text text-lg">Path to dbeaver</span>
				</label>
				<div class="m-2">
					<input
						id="dbeaver_path"
						type="text"
						autocomplete="off"
						autocorrect="off"
						autocapitalize="off"
						spellcheck="false"
						placeholder="DB path"
						class="input input-bordered w-full min-w-xs mb-2"
						bind:value={dbeaver_path}
					/>

					<div class="pl-2">
						Install <a
							class="link link-accent"
							href="https://dbeaver.io/"
							on:click|preventDefault={() => {
								open('https://dbeaver.io/');
							}}>dbeaver</a
						>
						to be able to open connection to database directly from Wombat<br /><br />
						MacOS:
						<pre class="pl-1">/Applications/DBeaver.app/Contents/MacOS/dbeaver</pre>

						Windows:
						<pre class="pl-1">C:\Program Files\DBeaver\dbeaver.exe</pre>
					</div>
				</div>
			</div>
		</div>

		<button class="btn btn-primary">Update path </button>
	</form>
</div>
