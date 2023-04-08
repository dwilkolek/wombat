<script lang="ts">
	import { execute } from '$lib/error-store';
	import { taskStore } from '$lib/task-store';
	import type { DbInstance } from '$lib/types';

	import dbeaver from '$lib/images/dbeaver-head.png';
	export let database: DbInstance | undefined;
</script>

{#if database}
	<div class="">
		{#if !$taskStore.find((t) => t.arn == database?.arn)}
			<button
				class="btn btn-sm btn-focus"
				disabled={!!$taskStore.find((t) => t.arn == database?.arn)}
				on:click={() => {
					execute('start_db_proxy', { db: database });
				}}>db proxy: {database.environment_tag.toUpperCase()}</button
			>
		{/if}
		{#if $taskStore.find((t) => t.arn == database?.arn)}
			<div class="flex flex-row items-center gap-1">
				{database.environment_tag.toUpperCase()} Port: {$taskStore.find(
					(t) => t.arn == database?.arn
				)?.port}
				<button
					class={`btn btn-sm btn-circle ${
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
					<img width="24" src={dbeaver} alt="open dbeaver" />
				</button>
			</div>
		{/if}
	</div>
{/if}
{#if !database}
	<div>N/A</div>
{/if}
