<script lang="ts">
	import { goto } from '$app/navigation';
	import { userStore } from '$lib/stores/user-store';
	import { open } from '@tauri-apps/api/shell';
	import { version } from '$app/environment';
	import { fetch } from '@tauri-apps/api/http';
	import { listen } from '@tauri-apps/api/event';
	import { exit } from '@tauri-apps/api/process';
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
	let userId: string = '';

	$: subscribe((userConfig) => {
		profile = userConfig?.last_used_profile ?? '';
		userId = userConfig?.id ?? '';
	});
	let loading = false;
	let buttonText = 'Start';
	listen<string>('message', (event) => {
		buttonText = event.payload;
	});

	listen<string>('KILL_ME', () => {
		exit(1);
	});
</script>

<svelte:head>
	<title>LOGIN</title>
	<meta name="description" content="Wombat" />
</svelte:head>
{#await subscribe then _}
	<div class="hero max-h-screen min-h-screen bg-base-200">
		<div class="hero-content flex-col">
			<div class="text-center">
				<h1 class="text-5xl font-medium">Hello!</h1>
				<p class="py-2">Wombat is friendly app that aims to make your life less miserable 😎</p>
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
				<div class="flex gap-1">
					<span>User Id:</span>
					<pre>{userId}</pre>
				</div>
			</div>
		</div>
	</div>
{/await}