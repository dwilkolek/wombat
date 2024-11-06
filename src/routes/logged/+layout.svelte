<script lang="ts">
	import { goto } from '$app/navigation';
	import WombatIcon from '$lib/images/128x128.png';
	import { page } from '$app/stores';
	import { invoke } from '@tauri-apps/api/core';
	import { execute } from '$lib/stores/error-store';
	import { userStore } from '$lib/stores/user-store';
	import { emit } from '@tauri-apps/api/event';
	import { version } from '$app/environment';
	import FeatureBtn from '$lib/componets/feature-btn.svelte';
	import BrowserExtensionDot from '$lib/componets/browser-extension-dot.svelte';
	import { featuresStore } from '$lib/stores/feature-store';
	interface Props {
		children?: import('svelte').Snippet;
	}

	let { children }: Props = $props();

	const logout = async (e: Event) => {
		try {
			e.preventDefault();
			await invoke('logout');
			emit('logged-out');
			goto('/');
		} catch (e) {
			console.log(e);
		}
	};
	let userConfig = $derived($userStore);
</script>

<div class="navbar bg-base-100 flex flex-row gap-2 justify-between px-3 sticky top-0 z-50">
	<div class="flex-none">
		<ul class="menu menu-horizontal px-1 gap-2">
			<li>
				<a
					class={$page.url.pathname === '/logged/apps' ? 'active pl-10 relative' : 'pl-10 relative'}
					href="/logged/apps"
				>
					<img class="h-10 absolute -left-2" alt="wombat" src={WombatIcon} />
					Apps
				</a>
			</li>
			<li>
				<a class={$page.url.pathname === '/logged/logs' ? 'active' : ''} href="/logged/logs"
					>Logs 🧐</a
				>
			</li>
			<li>
				<a
					class={$page.url.pathname === '/logged/lambda-apps' ? 'active' : ''}
					href="/logged/lambda-apps"
					>Lambda Apps
				</a>
			</li>
			{#if $featuresStore.tasksPage}
				<li>
					<a class={$page.url.pathname === '/logged/tasks' ? 'active' : ''} href="/logged/tasks"
						>Tasks
					</a>
				</li>
			{/if}
			<li>
				<a class={$page.url.pathname === '/logged/config' ? 'active' : ''} href="/logged/config"
					>Config</a
				>
			</li>
		</ul>
	</div>

	<div class="flex items-end gap-4">
		<div class="flex items-center gap-1 text-sm">
			App: {version}
		</div>
		<div class="flex items-center gap-1 text-sm">
			<span>Browser extension</span>
			<BrowserExtensionDot />
		</div>

		<FeatureBtn />

		<div class="flex items-center gap-2">
			<span>{userConfig.last_used_profile}</span>
		</div>
		<button
			data-umami-event="cache_refresh"
			data-umami-event-uid={userConfig.id}
			aria-label="Refresh cache"
			onclick={async () => {
				await execute('refresh_cache', undefined, true);
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
		<button
			data-umami-event="logout"
			data-umami-event-uid={userConfig.id}
			onclick={logout}
			onkeypress={logout}
			class="px-2 cursor-pointer"
			aria-label="Logout"
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
		</button>
	</div>
</div>
<div class="flex flex-col">
	<container style="min-height: calc(100vh - 72px)">
		{@render children?.()}
	</container>
</div>
