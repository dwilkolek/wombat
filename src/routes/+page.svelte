<script lang="ts">
	import { goto } from '$app/navigation';
	import { userStore } from '$lib/stores/user-store';
	import { version } from '$app/environment';
	import { listen } from '@tauri-apps/api/event';
	import { exit } from '@tauri-apps/plugin-process';
	import { invoke } from '@tauri-apps/api/core';
	import { availableAccountsStore } from '$lib/stores/available-accounts-store';
	import { envImportance } from '$lib/stores/env-store';
	import { BrowserExtensionState, type WombatAwsAccount } from '$lib/types';
	import { browserExtensionStatus } from '$lib/stores/browser-extension-status';
	import UpdateBtn from '$lib/components/update-btn.svelte';
	import FeatureBtn from '$lib/components/feature-btn.svelte';
	import BrowserExtensionDot from '$lib/components/browser-extension-dot.svelte';

	const { wombatAwsAccounts } = availableAccountsStore;
	let { login } = userStore;

	let account: WombatAwsAccount | undefined = $state();
	let userId: string | undefined = $state($userStore.id);
	let loading = $state(false);
	let buttonText = $state('Start');

	$effect(() => {
		if ($wombatAwsAccounts.length > 0 && $userStore.id) {
			const found = $wombatAwsAccounts.find((p) => p.id === $userStore.last_used_profile);
			if (found) {
				account = found;
			} else if (!account) {
				account = $wombatAwsAccounts[0];
			}
		}
	});

	listen<string>('message', (event) => {
		buttonText = event.payload;
	});

	function checkDependencies() {
		return invoke<
			Array<{
				name: string;
				ok: boolean;
				required: boolean;
				version_or_error: string;
			}>
		>('check_dependencies');
	}
	let dependenciesPromise = $state(checkDependencies());
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
				{#each deps as dep (dep)}
					<div class="flex items-center gap-1 text-sm">
						{#if dep.ok}
							<div class="bg-lime-500 w-2 h-2 rounded {!dep.required ? 'opacity-50' : ''}"></div>
						{:else}
							<div class="bg-rose-500 w-2 h-2 rounded {!dep.required ? 'opacity-50' : ''}"></div>
						{/if}
						<span>
							{dep.name} :
						</span>
						<span class="">
							{dep.version_or_error}
						</span>
					</div>
				{/each}
			{:catch error}
				<p>Failed: {error}</p>
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
		<h1 class="font-bold text-amber-400">Quokka chrome extension</h1>
		<div class="flex flex-col gap-1 text-sm">
			<ul class="list-disc ml-6">
				<li>automates <span class="text-orange-300">aws sso login</span> process</li>
				<li>automates github sign-in confirmation process</li>
				<li>enables proxying to lambda services like commenting service</li>
				<li>closes page after confirming identity by Snowflake JDBC driver</li>
			</ul>
			<a
				target="_blank"
				href="https://chromewebstore.google.com/detail/quokka/genpoikemhehdicnplfojdolhdhofonp"
				>👉 <span class="underline text-amber-300 hover:text-amber-500">Chrome web store</span></a
			>
		</div>
	</div>
	<div class="hero-content flex-col">
		<div class="text-center">
			<h1 class="text-5xl font-medium">Hello!</h1>
			<p class="py-6">Wombat is friendly app that aims to make your life less miserable 😎</p>
		</div>
		<div class="card shrink-0 w-full shadow-2xl bg-base-100">
			<div class="card-body">
				<form
					onsubmit={async (e) => {
						try {
							e.preventDefault();
							loading = true;
							await login(account);
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
							<span class="label-text">AWS Account</span>
						</label>
						<div class="flex items-center gap-2">
							{#if userId}
								<select class="select w-full" bind:value={account}>
									{#each $wombatAwsAccounts as wombatAwsAccount (wombatAwsAccount.id)}
										<option value={wombatAwsAccount}>
											{wombatAwsAccount.name}
											{#if wombatAwsAccount.support_level == 'Full'}✅{/if}
											{#if wombatAwsAccount.support_level == 'Partial'}⚠️{/if}
											{#if wombatAwsAccount.support_level == 'None'}🚫{/if}
										</option>
									{/each}
								</select>
							{/if}
							<button
								class="btn btn-square mb-1"
								data-umami-event="reload_aws_config"
								data-umami-event-uid={userId}
								aria-label="Reload AWS profiles"
								onclick={async (e) => {
									try {
										e.preventDefault();
										await invoke('reload_aws_config');
										await availableAccountsStore.refresh();
									} catch (e) {
										console.error(e);
									} finally {
										dependenciesPromise = checkDependencies();
									}
								}}
								><svg
									xmlns="http://www.w3.org/2000/svg"
									fill="none"
									viewBox="0 0 24 24"
									stroke-width="1.5"
									stroke="currentColor"
									class="w-4 h-4"
								>
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99"
									/>
								</svg>
							</button>
						</div>
					</div>
					<div class="mt-2">
						{#if account}
							<div class="flex flex-col gap-1 pl-2 text-sm">
								{#if account.single_source_profile}
									<span class="font-bold">This account is using single SSO profile.</span>
									<span
										>Environments: {[
											...new Set(
												Object.values(account.sso_profiles)
													.flat()
													.map((sso) => sso.env)
											)
										]
											.sort((a, b) => envImportance[a] - envImportance[b])
											.join(', ')}</span
									>
									<span
										>Infra profiles: {Object.values(account.sso_profiles)
											.flat()
											.reduce((acc, sso) => acc + sso.infra_profiles.length, 0)}</span
									>
								{:else}
									{#each Object.values(account.sso_profiles)
										.flat()
										.sort((a, b) => envImportance[a.env] - envImportance[b.env]) as sso_profile (sso_profile.profile_name)}
										<div class="flex gap-1">
											{#if sso_profile.support_level == 'Full'}✅{/if}
											{#if sso_profile.support_level == 'Partial'}⚠️{/if}
											{#if sso_profile.support_level == 'None'}🚫{/if}
											<b>{sso_profile.profile_name}</b>({sso_profile.infra_profiles.length} infra profiles)
										</div>
									{/each}
								{/if}
							</div>
						{/if}
					</div>
					{#await dependenciesPromise then deps}
						{#if deps.filter((d) => d.required).every((d) => d.ok)}
							<div class="form-control mt-6 flex justify-center">
								<button
									data-umami-event="login"
									data-umami-event-uid={userId}
									class="btn btn-accent w-full"
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
									class="btn btn-warning w-full"
									type="button"
									onclick={() => {
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
				<FeatureBtn hideProfileName={true} />
				<span class="text-sm">User Id: {userId}</span>
			</div>
		</div>
	</div>
</div>
