<script lang="ts">
	import '$lib/styles.css';

	import { page } from '$app/stores';
	import type { TeamPage } from './+page';
	export let data: TeamPage;
</script>

<div class="">
	<nav class="flex gap-2">
        <ul class="menu menu-horizontal px-1 gap-2">
			<li>
				<a
					class={$page.url.pathname === `/teams/${data.team}` ? 'active' : ''}
					href={`/teams/${data.team}`}
				>
					Apps 🚀
				</a>
			</li>
			<li>
				<a class={$page.url.pathname === `/teams/${data.team}/logs` ? 'active' : ''} href={`/teams/${data.team}/logs`}
					>Logs 🧐</a
				>
			</li>
			<li>
				<a class={$page.url.pathname === `/teams/${data.team}/ecs` ? 'active' : ''} href={`/teams/${data.team}/ecs`}
					>Services (ECS)
				</a>
			</li>
			<li>
				<a class={$page.url.pathname === `/teams/${data.team}/rds` ? 'active' : ''} href={`/teams/${data.team}/rds`}
					>Databases (RDS)</a
				>
			</li>
			<!-- <li>
				<a class={$page.url.pathname === '/logged/config' ? 'active' : ''} href="/logged/config"
					>Config</a
				>
			</li> -->
		</ul>
    </nav>
	<main>
		<slot />
	</main>
</div>