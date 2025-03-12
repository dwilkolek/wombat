<script lang="ts">
	import UserSessionProxyBtn from '$lib/componets/user-session-proxy-btn.svelte';
	import { TaskStatus, taskStore } from '$lib/stores/task-store';
</script>

<svelte:head>
	<title>Tasks</title>
	<meta name="description" content="Wombat" />
</svelte:head>
<div class="p-2 px-4">
	<UserSessionProxyBtn />
</div>
<div class="bg-base-100 flex flex-row justify-between px-2 sticky top-[68px] z-40">
	<div class="overflow-x-auto w-full">
		<table class="table table-zebra">
			<!-- head -->
			<thead>
				<tr>
					<th>Arn</th>
					<th>Port</th>
					<th>Authentication interceptor</th>
					<th>Status</th>
				</tr>
			</thead>
			<tbody>
				{#each $taskStore as task}
					<tr>
						<th>{task.arn}</th>
						<td>{task.port}</td>
						<td>
							{#if task.proxyAuthConfig}
								{task.proxyAuthConfig?.authType}:&nbsp;{task.proxyAuthConfig?.jepsenClientId ??
									task.proxyAuthConfig?.basicUser ??
									'?'}
							{/if}</td
						>
						<td>
							{#if task.status == TaskStatus.RUNNING}
								running
								<button
									onclick={async (e) => {
										e.preventDefault();
										await taskStore.stopTask(task.arn);
									}}>X</button
								>
							{/if}
							{#if task.status == TaskStatus.FAILED}
								failed
							{/if}
							{#if task.status == TaskStatus.STARTING}
								starting
							{/if}
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
</div>
