<script lang="ts">
	import type { ProxyEventMessage } from '$lib/stores/task-store';

	export let task: ProxyEventMessage | undefined;
</script>

{#if task}
	{#if task.status == 'STARTING'}
		<div class={`grow flex items-center gap-1 text-amber-300 `}>
			{#if task.jepsen_config?.client_id}
				<div class={`bg-lime-600 h-[2px] w-full rounded-full animate-pulse`} />
				<svg
					xmlns="http://www.w3.org/2000/svg"
					fill="none"
					viewBox="0 0 24 24"
					stroke-width="1.5"
					stroke="currentColor"
					class="min-w-4 min-h-4"
				>
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						d="M13.5 10.5V6.75a4.5 4.5 0 1 1 9 0v3.75M3.75 21.75h10.5a2.25 2.25 0 0 0 2.25-2.25v-6.75a2.25 2.25 0 0 0-2.25-2.25H3.75a2.25 2.25 0 0 0-2.25 2.25v6.75a2.25 2.25 0 0 0 2.25 2.25Z"
					/>
				</svg>

				<span class="text-sm"> {task.jepsen_config?.client_id}</span>
				<div class={`bg-lime-600 h-[2px] w-full rounded-full animate-pulse`} />
			{:else}
				<div class={`bg-lime-600 h-[2px] w-full rounded-full animate-pulse`} />
			{/if}
		</div>
	{/if}

	{#if task.status == 'STARTED'}
		<div class={`grow flex items-center gap-1 text-amber-300`}>
			{#if task.jepsen_config?.client_id}
				<div class={`bg-lime-600 h-[2px] w-full rounded-full`} />
				<svg
					xmlns="http://www.w3.org/2000/svg"
					fill="none"
					viewBox="0 0 24 24"
					stroke-width="1.5"
					stroke="currentColor"
					class="min-w-4 min-h-4"
				>
					<path
						stroke-linecap="round"
						stroke-linejoin="round"
						d="M13.5 10.5V6.75a4.5 4.5 0 1 1 9 0v3.75M3.75 21.75h10.5a2.25 2.25 0 0 0 2.25-2.25v-6.75a2.25 2.25 0 0 0-2.25-2.25H3.75a2.25 2.25 0 0 0-2.25 2.25v6.75a2.25 2.25 0 0 0 2.25 2.25Z"
					/>
				</svg>

				<span class="text-sm"> {task.jepsen_config?.client_id}</span>
				<div class={`bg-lime-600 h-[2px] w-full rounded-full`} />
			{:else}
				<div class={`bg-lime-600 h-[2px] w-full rounded-full`} />
			{/if}
		</div>
	{/if}
{:else}
	<div class={`grow flex items-center`}>
		<div class={`bg-base-100 h-[2px] w-full rounded-full`} />
	</div>
{/if}
