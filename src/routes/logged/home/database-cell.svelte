<script lang="ts">
	import { execute } from '$lib/error-store';
	import { taskStore } from '$lib/task-store';
	import { writeText } from '@tauri-apps/api/clipboard';
	import type { DatabaseCredentials, DbInstance } from '$lib/types';
	import { ask } from '@tauri-apps/api/dialog';
	import { userStore } from '$lib/user-store';
	import { invoke } from '@tauri-apps/api/tauri';
	import DbSecretBtn from '$lib/db-secret-btn.svelte';
	export let database: DbInstance | undefined;
</script>

{#if database}
	<div class="flex flex-row items-center gap-1">
		<DbSecretBtn database={database} />
		{#if !$taskStore.find((t) => t.arn == database?.arn)}
			<button
				class="flex flex-row gap-1"
				disabled={!!$taskStore.find((t) => t.arn == database?.arn)}
				on:click={() => {
					execute('start_db_proxy', { db: database });
				}}
			>
				<svg
					xmlns="http://www.w3.org/2000/svg"
					viewBox="0 0 20 20"
					fill="currentColor"
					class="w-5 h-5"
				>
					<path
						d="M6.3 2.841A1.5 1.5 0 004 4.11V15.89a1.5 1.5 0 002.3 1.269l9.344-5.89a1.5 1.5 0 000-2.538L6.3 2.84z"
					/>
				</svg>
				{database.engine}</button
			>
		{/if}
		{#if $taskStore.find((t) => t.arn == database?.arn)}
			<button on:click={async () => {
				await execute('stop_job', { arn: database?.arn });
			}}>
				<svg
					xmlns="http://www.w3.org/2000/svg"
					viewBox="0 0 20 20"
					fill="currentColor"
					class="w-5 h-5"
				>
					<path
						d="M5.75 3a.75.75 0 00-.75.75v12.5c0 .414.336.75.75.75h1.5a.75.75 0 00.75-.75V3.75A.75.75 0 007.25 3h-1.5zM12.75 3a.75.75 0 00-.75.75v12.5c0 .414.336.75.75.75h1.5a.75.75 0 00.75-.75V3.75a.75.75 0 00-.75-.75h-1.5z"
					/>
				</svg>
			</button>
			{#if $userStore.dbeaver_path}
				<button
					class={`link link-success ${
						!$taskStore.find((t) => t.arn == database?.arn) ? 'opacity-25' : ''
					}`}
					disabled={!$taskStore.find((t) => t.arn == database?.arn)}
					on:click={() => {
						execute(
							'open_dbeaver',
							{
								db: database,
								port: $taskStore.find((t) => t.arn == database?.arn)?.port
							},
							false
						);
					}}
				>
					{database.environment_tag.toUpperCase()} @ :{$taskStore.find(
						(t) => t.arn == database?.arn
					)?.port}
				</button>
			{/if}
		{/if}
	</div>
{/if}
{#if !database}
	<div>N/A</div>
{/if}
