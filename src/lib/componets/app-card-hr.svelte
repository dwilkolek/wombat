<script lang="ts">
	import type { ProxyEventMessage } from '$lib/stores/task-store';

	export let task: ProxyEventMessage | undefined;
</script>

{#if task}
	{#if task.status == 'STARTING'}
	<div class={`grow flex gap-1 text-amber-300 relative items-stretch animate-pulse`}>
		{#if task.proxy_auth_config}
			<span class="text-xs absolute bottom-0 left-4 w-full">{task.proxy_auth_config?.authType}:&nbsp;{task.proxy_auth_config?.jepsenClientId ?? task.proxy_auth_config?.basicUser ?? '?'}</span>
			<div class={`bg-lime-600 h-[2px] w-full rounded-full`} />
		{:else}
			<div class={`bg-lime-600 h-[2px] w-full rounded-full`} />
		{/if}
	</div>
	{/if}

	{#if task.status == 'STARTED'}
		<div class={`grow flex gap-1 text-amber-300 relative items-stretch`}>
			{#if task.proxy_auth_config}
				<span class="text-xs absolute bottom-0 left-4 w-full">{task.proxy_auth_config?.authType}:&nbsp;{task.proxy_auth_config?.jepsenClientId ?? task.proxy_auth_config?.basicUser ?? '?'}</span>
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
