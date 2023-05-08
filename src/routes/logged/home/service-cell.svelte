<script lang="ts">
	import { execute } from '$lib/error-store';
	import { homeStore } from '$lib/home-store';
	import { taskStore } from '$lib/task-store';
	import { AwsEnv, type ServiceDetails } from '$lib/types';
	import { open } from '@tauri-apps/api/shell';

	$: entries = Object.values(
		$homeStore.find((e) => e.tracked_name == service?.name)?.services ?? []
	) as ServiceDetails[];
	$: lower_rank_env =
		service?.env == AwsEnv.PROD
			? AwsEnv.DEMO
			: service?.env == AwsEnv.DEMO
			? AwsEnv.DEV
			: undefined;
	$: lower_rank_service_version = entries.find((s) => s.env == lower_rank_env)?.version;
	export let service: ServiceDetails | undefined;
</script>

{#if service}
	<div class="flex flex-row gap-2 items-start pr-4">
		{#if $taskStore.find((t) => t.arn == service?.arn)}
			<button
				on:click={async () => {
					await execute('stop_job', { arn: service?.arn });
				}}
			>
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
			<button
				class={`link ${
					lower_rank_service_version &&
					lower_rank_service_version != 'latest' &&
					lower_rank_service_version != service.version
						? 'link-warning'
						: 'link-success'
				}`}
				on:click|preventDefault={() => {
					open('http://localhost:' + $taskStore.find((t) => t.arn == service?.arn)?.port);
				}}
			>
				{service.version} @ :{$taskStore.find((t) => t.arn == service?.arn)?.port}</button
			>
		{/if}
		{#if !$taskStore.find((t) => t.arn == service?.arn)}
			<button
				class="flex flex-row gap-1"
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
						d="M6.3 2.841A1.5 1.5 0 004 4.11V15.89a1.5 1.5 0 002.3 1.269l9.344-5.89a1.5 1.5 0 000-2.538L6.3 2.84z"
					/>
				</svg>

				<span
					class={`${
						lower_rank_service_version &&
						lower_rank_service_version != 'latest' &&
						lower_rank_service_version != service.version
							? 'text-warning'
							: ''
					}`}>{service.version ?? '??'}</span
				>
			</button>
		{/if}
	</div>
{/if}
{#if !service}
	<div />
{/if}
