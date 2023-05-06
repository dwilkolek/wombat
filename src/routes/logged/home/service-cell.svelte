<script lang="ts">
	import { execute } from '$lib/error-store';
	import { homeStore } from '$lib/home-store';
	import { taskStore } from '$lib/task-store';
	import { AwsEnv, type ServiceDetails } from '$lib/types';
	import { open } from '@tauri-apps/api/shell';

	$: entries = Object.values($homeStore.find(e => e.tracked_name == service?.name)?.services ?? []) as ServiceDetails[];
	$: lower_rank_env =
		service?.env == AwsEnv.PROD ? AwsEnv.DEMO : service?.env == AwsEnv.DEMO ? AwsEnv.DEV : undefined;
	$: lower_rank_service_version = entries.find(s => s.env == lower_rank_env)?.version;
	export let service: ServiceDetails | undefined;
</script>
{#if service}

	<div class="flex flex-row gap-2 items-start pr-4">
		<span
			class={`${
				lower_rank_service_version &&
				lower_rank_service_version != 'latest' &&
				lower_rank_service_version != service.version
					? 'text-warning'
					: ''
			}`}
		>
	
		{service.version ?? '??'}</span
		>
		{#if !$taskStore.find((t) => t.arn == service?.arn)}
			<button
				on:click={() => {
					execute('start_service_proxy', { service });
				}}
			>
				<svg
					xmlns="http://www.w3.org/2000/svg"
					viewBox="0 0 20 20"
					fill="currentColor"
					class="w-5 h-5"
				>
					<path
						d="M4.25 2A2.25 2.25 0 002 4.25v2a.75.75 0 001.5 0v-2a.75.75 0 01.75-.75h2a.75.75 0 000-1.5h-2zM13.75 2a.75.75 0 000 1.5h2a.75.75 0 01.75.75v2a.75.75 0 001.5 0v-2A2.25 2.25 0 0015.75 2h-2zM3.5 13.75a.75.75 0 00-1.5 0v2A2.25 2.25 0 004.25 18h2a.75.75 0 000-1.5h-2a.75.75 0 01-.75-.75v-2zM18 13.75a.75.75 0 00-1.5 0v2a.75.75 0 01-.75.75h-2a.75.75 0 000 1.5h2A2.25 2.25 0 0018 15.75v-2zM7 10a3 3 0 116 0 3 3 0 01-6 0z"
					/>
				</svg>
			</button>
		{/if}
		{#if $taskStore.find((t) => t.arn == service?.arn)}
			<button
				class="underline"
				on:click|preventDefault={() => {
					open('http://localhost:' + $taskStore.find((t) => t.arn == service?.arn)?.port);
				}}
			>
				At port: {$taskStore.find((t) => t.arn == service?.arn)?.port}</button
			>
		{/if}
	</div>
{/if}
{#if !service}
	<div>N/A</div>
{/if}
