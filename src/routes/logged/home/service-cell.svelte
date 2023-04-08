<script lang="ts">
	import { execute } from '$lib/error-store';
	import { homeStore } from '$lib/home-store';
	import { taskStore } from '$lib/task-store';
	import { AwsEnv, type ServiceDetails } from '$lib/types';
	import { invoke } from '@tauri-apps/api';

	$: entries = $homeStore;
	$: lower_rank_env =
		service?.env == AwsEnv.PROD ? AwsEnv.DEMO : service?.env == AwsEnv.DEMO ? AwsEnv.DEV : '';
	export let service: ServiceDetails | undefined;
</script>

{#if service}
	<div class="flex flex-row gap-2 items-start pr-4">
		<span
			class={`font-bold ${
				entries[service?.name][lower_rank_env] &&
				entries[service?.name][lower_rank_env]?.service?.version != 'latest' &&
				entries[service?.name][lower_rank_env]?.service?.version !=
					entries[service?.name][service?.env]?.service?.version
					? 'text-warning'
					: ''
			}`}
		>
			{service?.version ?? '??'}</span
		>
		{#if !$taskStore.find((t) => t.arn == service?.arn)}
			<button
				on:click={() => {
					invoke('start_service_proxy', { service });
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
