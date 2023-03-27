<script lang="ts">
	import { open } from '@tauri-apps/api/shell';
	import Icon from '$lib/images/32x32.png';
	import { Environment, state } from '../store';
	import { page } from '$app/stores';
	const environments = Object.keys(Environment).map((env) => env as Environment);
	const env = state.env;
	const profile = state.profile;
	const openGithubPage = () => {
		open('https://github.com/dwilkolek/wombat');
	};
</script>

<div class="flex flex-col">
	<div class="navbar bg-base-100 flex flex-row gap-2 justify-between">
		<div class="flex-none">
			<ul class="menu menu-horizontal px-1">
				<li><a href="/"><img class="h-[32px]" alt="wombat" src={Icon} /></a></li>
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
			<input
				type="text"
				disabled={true}
				placeholder="AWS profile"
				class="input input-bordered w-full max-w-xs text-focus"
				bind:value={$profile}
			/>
			<select class="select select-bordered" bind:value={$env}>
				{#each environments as env}
					<option value={env}>{env}</option>
				{/each}
			</select>
		</div>
	</div>

	<container class="overflow-y-auto" style="height: calc(100vh - 112px);">
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
