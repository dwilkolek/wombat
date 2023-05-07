<script lang="ts">
	import { goto } from '$app/navigation';
	import Icon from '$lib/images/32x32.png';
	import { page } from '$app/stores';
	import { invoke } from '@tauri-apps/api/tauri';
	import { execute } from '$lib/error-store';
	import { homeStore } from '$lib/home-store';
	import type { UserConfig } from '$lib/types';
	import { clusterStore } from '$lib/cluster-store';

	const logout = async () => {
		await invoke('logout');
		goto('/');
	};
	let userConfig = invoke<UserConfig>('user_config');
</script>

<div class="navbar bg-base-100 flex flex-row gap-2 justify-between">
	<div class="flex-none">
		<ul class="menu menu-horizontal px-1">
			<li>
				<a class={$page.url.pathname === '/logged/home' ? 'active' : ''} href="/logged/home">
					<img class="h-full" alt="wombat" src={Icon} />
				</a>
			</li>

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
				<a class={$page.url.pathname === '/logged/config' ? 'active' : ''} href="/logged/config"
					>Config</a
				>
			</li>
		</ul>
	</div>

	<div class="flex gap-4">
		{#await userConfig then { last_used_profile }}
			<h6>{last_used_profile}</h6>
		{/await}
		<button
			on:click={async () => {
				await execute('refresh_cache', undefined, true);
				await clusterStore.refresh();
				await homeStore.refresh(false);
			}}
			><svg
				xmlns="http://www.w3.org/2000/svg"
				fill="none"
				viewBox="0 0 24 24"
				stroke-width="1.5"
				stroke="currentColor"
				class="w-6 h-6"
			>
				<path
					stroke-linecap="round"
					stroke-linejoin="round"
					d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99"
				/>
			</svg>
		</button>
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
	<container style="min-height: calc(100vh - 72px)">
		<slot />
	</container>
</div>
