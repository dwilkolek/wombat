<script lang="ts">
	import { goto } from '$app/navigation';
	import { userStore } from '$lib/user-store';
	import { open } from '@tauri-apps/api/shell';
	import { version } from '$app/environment';
	import { fetch } from '@tauri-apps/api/http';
	$: latest = fetch('https://api.github.com/repos/dwilkolek/wombat/releases/latest').then((r) => {
		return (r as any).data.html_url.split('/v').at(-1) as string;
	});
	const openGithubPage = () => {
		open('https://github.com/dwilkolek/wombat');
	};
	const openGithubPageRelease = () => {
		open('https://github.com/dwilkolek/wombat/releases/latest');
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
								{loading ? 'Preloading...' : 'Start'}</button
							>
						</div>
					</form>
				</div>
			</div>
			<div class="flex flex-col justify-center items-center gap-2 my-2">
				{#await latest then latest}
					{#if (latest ?? '0.0.0')
						.split('.')
						.map((v, i) => parseInt(v) * Math.pow(1000, 3 - i))
						.reduce((acc, v) => acc + v, 0) > version
							.split('.')
							.map((v, i) => parseInt(v) * Math.pow(1000, 3 - i))
							.reduce((acc, v) => acc + v, 0)}
						<a
							class="underline text-accent"
							href="https://github.com/dwilkolek/wombat/releases/latest"
							on:click|preventDefault={() => {
								openGithubPageRelease();
							}}
							target="_blank"
							>New version v{latest} available!
						</a>
					{/if}
				{/await}
				<div>
					<span>Source code:</span>
					<a
						class="underline"
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
	</div>
{/await}
