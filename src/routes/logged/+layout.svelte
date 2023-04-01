<script lang="ts">
	import { goto } from '$app/navigation';
	import Icon from '$lib/images/32x32.png';
	import { page } from '$app/stores';
	import { invoke } from '@tauri-apps/api/tauri';
	import { Env, type UserConfig } from '../types';
	import { currentEnv } from '../env-store';
	const logout = async () => {
		await invoke('logout');
		goto('/');
	};

	let userConfig = invoke<UserConfig>('user_config');
	let envs = Object.keys(Env);
</script>

<div class="navbar bg-base-100 flex flex-row gap-2 justify-between">
	<div class="flex-none">
		<ul class="menu menu-horizontal px-1">
			<li><img class="h-full" alt="wombat" src={Icon} /></li>
			<li>
				<a class={$page.url.pathname === '/logged/ecs' ? 'active' : ''} href="/logged/ecs"
					>Services (ECS)</a
				>
			</li>
			<li>
				<a class={$page.url.pathname === '/logged/rds' ? 'active' : ''} href="/logged/rds"
					>Databases (RDS)</a
				>
			</li>
			<li>
				<a class={$page.url.pathname === '/logged/profile' ? 'active' : ''} href="/logged/profile"
					>Profile</a
				>
			</li>
		</ul>
	</div>

	<div class="flex gap-2">
		<select class="select select-bordered" bind:value={$currentEnv}>
			{#each envs as env}
				<option value={env}>{env}</option>
			{/each}
		</select>
		{#await userConfig then { last_used_profile }}
			<h6>{last_used_profile}</h6>
		{/await}
		<a
			href="/"
			on:click|preventDefault={logout}
			on:keypress|preventDefault={logout}
			class="px-2 cursor-pointer"
		>
			<svg
				xmlns="http://www.w3.org/2000/svg"
				fill="none"
				viewBox="0 0 24 24"
				stroke-width="1.5"
				stroke="currentColor"
				class="w-6 h-6"
				><path
					stroke-linecap="round"
					stroke-linejoin="round"
					d="M15.75 9V5.25A2.25 2.25 0 0013.5 3h-6a2.25 2.25 0 00-2.25 2.25v13.5A2.25 2.25 0 007.5 21h6a2.25 2.25 0 002.25-2.25V15M12 9l-3 3m0 0l3 3m-3-3h12.75"
				/>
			</svg>
		</a>
	</div>
</div>
<div class="flex flex-col">
	<container class="min-h-screen">
		<slot />
	</container>
</div>
