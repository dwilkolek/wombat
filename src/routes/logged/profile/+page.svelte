<script lang="ts">
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
			<div class="border-2 text-sm rounded p-4">
				<label class="label p-0 m-0" for="dbeaver_path">
					<span class="label-text text-lg">Path to dbeaver</span>
				</label>
				<div class="m-2">
					<input
						id="dbeaver_path"
						type="text"
						placeholder="DB path"
						class="input input-bordered w-full min-w-xs mb-2"
						bind:value={dbeaver_path}
					/>

					<div class="pl-2">
						MacOS:
						<pre class="pl-1">/Applications/DBeaver.app/Contents/MacOS/dbeaver</pre>

						Windows:
						<pre class="pl-1">C:\Program Files\DBeaver\dbeaver.exe</pre>
					</div>
				</div>
			</div>
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
