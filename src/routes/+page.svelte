<script lang="ts">
	import { goto } from '$app/navigation';
	import { userStore } from '$lib/stores/user-store';
	import { version } from '$app/environment';
	import { listen } from '@tauri-apps/api/event';
	import { exit } from '@tauri-apps/plugin-process';
	import { invoke } from '@tauri-apps/api/core';
	import { availableProfilesStore } from '$lib/stores/available-profiles-store';
	import { envImportance } from '$lib/stores/env-store';
	import { BrowserExtensionState, type WombatAwsProfile } from '$lib/types';
	import { browserExtensionStatus } from '$lib/stores/browser-extension-status';
	import UpdateBtn from '$lib/componets/update-btn.svelte';
	import FeatureBtn from '$lib/componets/feature-btn.svelte';
	import BrowserExtensionDot from '$lib/componets/browser-extension-dot.svelte';

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

	function checkDependencies() {
		return invoke<{ [key: string]: { Ok: string } & { Err: string } }>('check_dependencies');
	}
	let dependenciesPromise = checkDependencies();
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
				<BrowserExtensionDot />
				<span>browser extension : </span>
				<span class="">
					{#if $browserExtensionStatus.state != BrowserExtensionState.Disconnected}
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
				<li>enables proxying to lambda services like commenting service</li>
				<li>closes page after confirming identity by Snowflake JDBC driver</li>
			</ul>
			<a
				target="_blank"
				href="https://chromewebstore.google.com/detail/wombat-plugin/genpoikemhehdicnplfojdolhdhofonp"
				>👉 <span class="underline text-amber-300 hover:text-amber-500">Chrome web store</span></a
			>
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
							<div class="form-control mt-6">
								<button
									data-umami-event="recheck_dependencies"
									data-umami-event-uid={userId}
									class="btn btn-warning"
									type="button"
									on:click={() => {
										dependenciesPromise = checkDependencies();
									}}
								>
									Requirements not met. Check again!</button
								>
							</div>
						{/if}
					{/await}

					{#if $browserExtensionStatus.state === BrowserExtensionState.Outdated && ($browserExtensionStatus.version ?? '4.0').startsWith('4.')}
						<p class="text-amber-500 mt-1">Please use extension from Chrome web store</p>
					{/if}
				</form>
			</div>
		</div>
		<div class="flex flex-col justify-center items-center gap-2 my-2">
			<UpdateBtn />
			<div>
				<span>Source code:</span>
				<a class="underline" href="https://github.com/dwilkolek/wombat" target="_blank"
					>https://github.com/dwilkolek/wombat v{version}
				</a>
			</div>
			<div class="flex gap-1 items-center">
				<FeatureBtn />
				<span class="text-sm">User Id: {userId}</span>
			</div>
		</div>
	</div>
</div>
