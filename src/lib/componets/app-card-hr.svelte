<script lang="ts">
	import type { ProxyEventMessage } from '$lib/stores/task-store';

	export let task: ProxyEventMessage | undefined;
</script>

{#if task}
	{#if task.status == 'STARTING'}
		<div class={`grow flex items-center animate-pulse`}>
			<div class={`bg-lime-700 h-[2px] w-full rounded-full`} />
		</div>
	{/if}

	{#if task.status == 'STARTED'}
		<div class={`grow flex items-center`}>
			<div class={`bg-lime-600 h-[2px] w-full rounded-full`} />
		</div>
	{/if}
{:else}
	<div class={`grow flex items-center`}>
		<div class={`bg-base-100 h-[2px] w-full rounded-full`} />
	</div>
{/if}
