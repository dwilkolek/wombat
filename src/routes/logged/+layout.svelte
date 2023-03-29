<script lang="ts">
	import { open } from '@tauri-apps/api/shell';
	import { goto } from '$app/navigation';
	import Icon from '$lib/images/32x32.png';
	import { page } from '$app/stores';
	import { invoke } from '@tauri-apps/api/tauri';
	import type { UserConfig } from '../types';
	const logout = async () => {
		await invoke('logout');
		goto('/');
	};
	const openGithubPage = () => {
		open('https://github.com/dwilkolek/wombat');
	};
	let userConfig = invoke<UserConfig>('user_config');
</script>

<div class="flex flex-col">
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
			{#await userConfig then { last_used_profile }}
				{last_used_profile}
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

	<container class="overflow-y-auto" style="height: calc(100vh - 120px);">
		<slot />
	</container>
	<div class="flex justify-center gap-2 my-2">
		<span>Sourcecode:</span>
		<a
			href="https://github.com/dwilkolek/wombat"
			on:click|preventDefault={() => {
				openGithubPage();
			}}
			target="_blank"
			>https://github.com/dwilkolek/wombat
		</a>
	</div>
</div>
