<script lang="ts">
	import { goto } from '$app/navigation';
	import { userStore } from '$lib/user-store';
	import { open } from '@tauri-apps/api/shell';
	import { version } from '$app/environment';

	const openGithubPage = () => {
		open('https://github.com/dwilkolek/wombat');
	};
	let { subscribe, login } = userStore;
	let profile: string = '';

	$: subscribe((userConfig) => {
		profile = userConfig?.last_used_profile ?? '';
	});

	let loading = false;
</script>

<svelte:head>
	<title>LOGIN</title>
	<meta name="description" content="Wombat" />
</svelte:head>
{#await subscribe then _}
	<div class="hero max-h-screen min-h-screen bg-base-200">
		<div class="hero-content flex-col">
			<div class="text-center">
				<h1 class="text-5xl font-bold">Hello!</h1>
				<p class="py-6">Wombat is friendly app that aims to make your life less miserable 😎</p>
			</div>
			<div class="card flex-shrink-0 w-full max-w-sm shadow-2xl bg-base-100">
				<div class="card-body">
					<form
						on:submit|preventDefault={async () => {
							try {
								loading = true;
								await login(profile);
								loading = false;
								goto(`/logged/home`, { replaceState: true });
							} catch (e) {
								console.error(e);
								loading = false;
							}
						}}
					>
						<div class="form-control">
							<label class="label" for="aws-profile">
								<span class="label-text">AWS profile</span>
							</label>
							<input
								id="aws-profile"
								type="text"
								autocomplete="false"
								autocorrect="off"
								autocapitalize="off"
								spellcheck="false"
								placeholder="AWS profile"
								class="input input-bordered w-full max-w-xs"
								bind:value={profile}
								required
							/>
						</div>

						<div class="form-control mt-6">
							<button class="btn btn-accent" disabled={loading} type="submit">
								{loading ? 'Preloading...' : 'Get Start'}</button
							>
						</div>
					</form>
				</div>
			</div>
			<div class="flex justify-center gap-2 my-2">
				<span>Source code:</span>
				<a
					href="https://github.com/dwilkolek/wombat"
					on:click|preventDefault={() => {
						openGithubPage();
					}}
					target="_blank"
					>https://github.com/dwilkolek/wombat v{version}
				</a>
			</div>
		</div>
	</div>
{/await}
