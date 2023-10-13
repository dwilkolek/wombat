<script lang="ts">
	import { taskStore, type ProxyEventMessage } from '$lib/stores/task-store';
	import { envImportance } from '$lib/stores/cluster-store';
	import DbeaverBtn from './dbeaver-btn.svelte';
	import DbProxyStopBtn from './db-proxy-stop-btn.svelte';
	import ServiceProxyStopBtn from './service-proxy-stop-btn.svelte';
	$: taskGroups = $taskStore.reduce(
		(
			acc: { name: string; ecs: ProxyEventMessage[]; rds: ProxyEventMessage[] }[],
			task: ProxyEventMessage
		) => {
			if (!acc.find((t) => t.name == task.name)) {
				acc.push({ name: task.name, ecs: [], rds: [] });
			}

			const group = acc.find((t) => t.name == task.name)!!;
			if (task.proxy_type == 'ECS') {
				group.ecs.push(task);
				group.ecs.sort((a, b) => envImportance[b.env] - envImportance[a.env]);
			} else {
				group.rds.push(task);
				group.rds.sort((a, b) => envImportance[b.env] - envImportance[a.env]);
			}

			return acc;
		},
		[]
	);
</script>

<div class="flex gap-2 flex-col w-full">
	{#each taskGroups as taskGroup}
		<div class="flex flex-col gap-1">
			<h5 class="text-info text-md font-medium uppercase">
				{taskGroup.name}
			</h5>
			<div class="flex flex-col gap-2 pl-1">
				{#each taskGroup.ecs as task}
					<div class="flex gap-2 items-center text-sm">
						<ServiceProxyStopBtn service_arn={task.arn} />
						<span class="italic text-sm">Service {task.env}:</span>
						<div
							class="tooltip flex items-center text-amber-300 hover:text-amber-500 gap-1"
							data-tip={`Open ${task.name} in browser`}
						>
							<button
								class="link text-md"
								on:click|preventDefault={() => {
									open('http://localhost:' + task.port);
								}}
							>
								{task.port}</button
							>
						</div>
					</div>
				{/each}
				{#each taskGroup.rds as task}
					<div class="flex gap-2 items-center">
						<DbProxyStopBtn database_arn={task.arn} />
						<DbeaverBtn {task} />
					</div>
				{/each}
			</div>
		</div>
	{/each}
</div>
