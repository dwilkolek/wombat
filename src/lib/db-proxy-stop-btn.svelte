<script lang="ts">
	import dbeaver from '$lib/images/dbeaver-head.png';
	import { execute } from '$lib/error-store';
	import  type {  DbInstance } from '$lib/types';
	import { userStore } from './user-store';

	export let database: DbInstance;
	export let port: number
</script>
<div class="flex flex-row gap-1">
	<div class="tooltip" data-tip="Stop proxy to database">
		<button
			on:click={async () => {
				await execute('stop_job', { arn: database?.arn });
			}}
		>
			<div class="w-5 h-5 relative">
				<svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" 
				class="w-4 h-4 absolute text-info">
					<path stroke-linecap="round" stroke-linejoin="round" d="M20.25 6.375c0 2.278-3.694 4.125-8.25 4.125S3.75 8.653 3.75 6.375m16.5 0c0-2.278-3.694-4.125-8.25-4.125S3.75 4.097 3.75 6.375m16.5 0v11.25c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125V6.375m16.5 0v3.75m-16.5-3.75v3.75m16.5 0v3.75C20.25 16.153 16.556 18 12 18s-8.25-1.847-8.25-4.125v-3.75m16.5 0c0 2.278-3.694 4.125-8.25 4.125s-8.25-1.847-8.25-4.125" />
				</svg>
				
				<svg
					xmlns="http://www.w3.org/2000/svg"
					viewBox="0 0 20 20"
					fill="currentColor"
					class="w-3 h-3 absolute text-xs right-0 bottom-0 text-accent"
				>
				<path
				d="M5.75 3a.75.75 0 00-.75.75v12.5c0 .414.336.75.75.75h1.5a.75.75 0 00.75-.75V3.75A.75.75 0 007.25 3h-1.5zM12.75 3a.75.75 0 00-.75.75v12.5c0 .414.336.75.75.75h1.5a.75.75 0 00.75-.75V3.75a.75.75 0 00-.75-.75h-1.5z"
			/>
				</svg>
			</div>
		</button>

		<span>{database.engine_version ?? '??'}:{port}</span>
	</div>
	{#if $userStore.dbeaver_path}
		<div class="tooltip" data-tip="Open connection in dbeaver">
			<button
				class={`link link-success`}
				on:click={() => {
					execute(
						'open_dbeaver',
						{
							db: database,
							port
						},
						false
					);
				}}
			>
			<img width="24" src={dbeaver} alt="download icon" />
			</button>
		</div>
	{/if}
</div>