<script lang="ts">
	import { goto } from '$app/navigation';
	import { userStore } from '$lib/stores/user-store';
	import { open } from '@tauri-apps/api/shell';
	import { version } from '$app/environment';
	import { fetch } from '@tauri-apps/api/http';
	import { listen } from '@tauri-apps/api/event';
	import { exit } from '@tauri-apps/api/process';
	import { invoke } from '@tauri-apps/api';
	import { availableProfilesStore } from '$lib/stores/available-profiles-store';
	import { envImportance } from '$lib/stores/env-store';
	import type { WombatAwsProfile } from '$lib/types';
	import { browserExtensionStatus } from '$lib/stores/browser-extension-status';
	$: latest = fetch('https://api.github.com/repos/dwilkolek/wombat/releases/latest').then((r) => {
		return (
			(r as unknown as { data: undefined | { html_url: undefined | string } })?.data?.html_url ??
			'/v9.9.9'
		)
			.split('/v')
			.at(-1) as string;
	});
	const openGithubPage = () => {
		open('https://github.com/dwilkolek/wombat');
	};
	const openGithubPageRelease = () => {
		open('https://github.com/dwilkolek/wombat/releases/latest');
	};
	const openExtInstallGuide = () => {
		open(
			'https://developer.chrome.com/docs/extensions/get-started/tutorial/hello-world#load-unpacked'
		);
	};

	const { wombatAwsProfiles } = availableProfilesStore;
	let { login } = userStore;

	let profile: WombatAwsProfile | undefined;
	let userId: string = $userStore.id ?? '';
	let loading = false;
	let buttonText = 'Start';
	userStore.subscribe((user) => {
		wombatAwsProfiles.subscribe((profiles) => {
			profile = profiles.find((p) => p?.name == user.last_used_profile);
			userId = user.id ?? '';
		});
	});
	listen<string>('message', (event) => {
		buttonText = event.payload;
	});

	let dependenciesPromise = invoke<{ [key: string]: { Ok: string } & { Err: string } }>(
		'check_dependencies'
	);

	listen<string>('KILL_ME', () => {
		exit(1);
	});
</script>

<svelte:head>
	<title>LOGIN</title>
	<meta name="description" content="Wombat" />
</svelte:head>
<div class="hero max-h-screen min-h-screen bg-base-200">
	<div class="absolute left-4 top-4 p-2">
		<div class="flex flex-col gap-1">
			{#await dependenciesPromise then deps}
				{@const entries = Object.entries(deps).sort((a, b) => a[0].localeCompare(b[0]))}
				{#each entries as dep}
					<div class="flex items-center gap-1 text-sm">
						{#if dep[1].Ok}
							<div class="bg-lime-500 w-2 h-2 rounded" />
						{:else}
							<div class="bg-rose-500 w-2 h-2 rounded" />
						{/if}
						<span>
							{dep[0]} :
						</span>
						<span class="">
							{dep[1].Ok ?? dep[1].Err}
						</span>
					</div>
				{/each}
			{/await}
			<div class="flex items-center gap-1 text-sm">
				{#if $browserExtensionStatus.connected}
					{#if $browserExtensionStatus.version == version}
						<div class="bg-lime-500 w-2 h-2 rounded" />
					{:else}
						<div class="bg-amber-500 w-2 h-2 rounded" />
					{/if}
				{:else}
					<div class="bg-rose-500 w-2 h-2 rounded" />
				{/if}
				<span>browser extension : </span>
				<span class="">
					{#if $browserExtensionStatus.connected}
						v{$browserExtensionStatus.version}
					{:else}
						Disconnected
					{/if}
				</span>
			</div>
		</div>
	</div>
	<div class="absolute right-4 top-4 p-2">
		<h1 class="font-bold text-amber-400">Chrome extension</h1>
		<div class="flex flex-col gap-1 text-sm">
			<ul class="list-disc ml-6">
				<li>automates <span class="text-orange-300">aws sso login</span> process</li>
				<li>automates github sign-in confirmation process</li>
				<li>enables proxing to lambda services like commenting service</li>
				<li>closes page after confirming identity by Snowflake JDBC driver</li>
			</ul>

			<a
				on:click|preventDefault={() => {
					openExtInstallGuide();
				}}
				target="_blank"
				href="https://developer.chrome.com/docs/extensions/get-started/tutorial/hello-world#load-unpacked"
				>👉 <span class="underline text-amber-300 hover:text-amber-500">Installation guide</span></a
			>
			{#await invoke('chrome_extension_dir') then dir}
				<pre class="text-xs ml-6">{dir}</pre>
			{/await}
		</div>
	</div>
	<div class="hero-content flex-col">
		<div class="text-center">
			<h1 class="text-5xl font-medium">Hello!</h1>
			<p class="py-6">Wombat is friendly app that aims to make your life less miserable 😎</p>
		</div>
		<div class="card flex-shrink-0 w-full shadow-2xl bg-base-100">
			<div class="card-body">
				<form
					on:submit|preventDefault={async () => {
						try {
							loading = true;
							await login(profile);
							loading = false;
							goto(`/logged/apps`, { replaceState: true });
						} catch (e) {
							console.error(e);
							buttonText = 'Start Again';
							loading = false;
						}
					}}
				>
					<div class="form-control">
						<label class="label" for="aws-profile">
							<span class="label-text">AWS profile</span>
						</label>
						{#if userId}
							<select class="select select-bordered w-full" bind:value={profile}>
								{#each $wombatAwsProfiles as wombatAwsProfile}
									<option value={wombatAwsProfile}>
										{wombatAwsProfile.name}
										{#if wombatAwsProfile.support_level == 'Full'}✅{/if}
										{#if wombatAwsProfile.support_level == 'Partial'}⚠️{/if}
										{#if wombatAwsProfile.support_level == 'None'}🚫{/if}
									</option>
								{/each}
							</select>
						{/if}
					</div>
					<div class="mt-2">
						{#if profile}
							<div class="flex flex-col gap-1 pl-2 text-sm">
								{#if profile.single_source_profile}
									<span class="font-bold">This profile is using single SSO profile.</span>
									<span
										>Environments: {Object.values(profile.sso_profiles)
											.sort((a, b) => envImportance[a.env] - envImportance[b.env])
											.map((sso) => sso.env)
											.join(', ')}</span
									>
									<span
										>Infra profiles: {Object.values(profile.sso_profiles)[0].infra_profiles
											.length}</span
									>
								{:else}
									{#each Object.values(profile.sso_profiles).sort((a, b) => envImportance[a.env] - envImportance[b.env]) as sso_profiles}
										<div class="flex gap-1">
											{#if sso_profiles.support_level == 'Full'}✅{/if}
											{#if sso_profiles.support_level == 'Partial'}⚠️{/if}
											{#if sso_profiles.support_level == 'None'}🚫{/if}
											<b>{sso_profiles.profile_name}</b>({sso_profiles.infra_profiles.length} infra profiles)
										</div>
									{/each}
								{/if}
							</div>
						{/if}
					</div>
					{#await dependenciesPromise then deps}
						{#if !Object.entries(deps).some((v) => v[1].Err)}
							<div class="form-control mt-6">
								<button
									data-umami-event="login"
									data-umami-event-uid={userId}
									class="btn btn-accent"
									disabled={loading}
									type="submit"
								>
									{buttonText}</button
								>
							</div>
						{:else}
							<div class="text-rose-500">Required dependency is missing</div>
						{/if}
					{/await}
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
			<div class="flex gap-1">
				<span>User Id:</span>
				<pre>{userId}</pre>
			</div>
		</div>
	</div>
</div>
