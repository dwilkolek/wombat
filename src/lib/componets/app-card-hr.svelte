<script lang="ts">
	import { TaskStatus, type Task } from '$lib/stores/task-store';

	interface Props {
		task: Task | undefined;
	}

	let { task }: Props = $props();
</script>

{#if task}
	{#if task.status == TaskStatus.STARTING}
		<div class="grow flex gap-1 text-amber-300 relative items-stretch animate-pulse">
			{#if task.proxyAuthConfig}
				<span class="text-xs absolute bottom-0 left-4 w-full"
					>{task.proxyAuthConfig?.authType}:&nbsp;{task.proxyAuthConfig?.jepsenClientId ??
						task.proxyAuthConfig?.basicUser ??
						'?'}</span
				>
				<div class="bg-lime-600 h-[2px] w-full rounded"></div>
			{:else}
				<div class="bg-lime-600 h-[2px] w-full rounded"></div>
			{/if}
		</div>
	{/if}

	{#if task.status == TaskStatus.RUNNING}
		<div class="grow flex gap-1 text-amber-300 relative items-stretch">
			{#if task.proxyAuthConfig}
				<span class="text-xs absolute bottom-0 left-4 w-full"
					>{task.proxyAuthConfig?.authType}:&nbsp;{task.proxyAuthConfig?.jepsenClientId ??
						task.proxyAuthConfig?.basicUser ??
						'?'}</span
				>
				<div class="bg-lime-600 h-[2px] w-full rounded"></div>
			{:else}
				<div class="bg-lime-600 h-[2px] w-full rounded"></div>
			{/if}
		</div>
	{/if}

	{#if task.status == TaskStatus.FAILED}
		<div class="grow flex gap-1 text-amber-300 relative items-stretch">
			{#if task.proxyAuthConfig}
				<span class="text-xs absolute bottom-0 left-4 w-full"
					>{task.proxyAuthConfig?.authType}:&nbsp;{task.proxyAuthConfig?.jepsenClientId ??
						task.proxyAuthConfig?.basicUser ??
						'?'}</span
				>
				<div class="bg-rose-600 h-[2px] w-full rounded"></div>
			{:else}
				<div class="bg-rose-600 h-[2px] w-full rounded"></div>
			{/if}
		</div>
	{/if}
{:else}
	<div class="grow flex items-center">
		<div class="bg-base-100 h-[2px] w-full rounded"></div>
	</div>
{/if}
