<script lang="ts">
	import { endOfDay, format, startOfDay } from 'date-fns';
	import { clusterStore } from '$lib/stores/cluster-store';
	import { serviceStore } from '$lib/stores/service-store';
	import { invoke } from '@tauri-apps/api';
	import { beforeNavigate } from '$app/navigation';
	import { logStore } from '$lib/stores/log-store';
	import ServiceMultiselect from '$lib/componets/service-multiselect.svelte';
	import { userStore } from '$lib/stores/user-store';
	import JsonView from '$lib/componets/json-view.svelte';
	import { WebviewWindow } from '@tauri-apps/api/window';
	import TimerangeSelect from '$lib/componets/timerange-select.svelte';

	$: activeCluser = clusterStore.activeCluser;

	$: selectedServices = serviceStore.selectedServices;

	$: clusters = clusterStore.clusters;

	$: timerange = logStore.timerange;
	$: filterString = logStore.filterString;
	$: selectedLog = logStore.selectedLog;
	$: storeState = logStore.storeState;

	beforeNavigate(async () => {
		invoke('abort_find_logs', { reason: 'navigation' });
	});
	type LogFilter = {
		id: number;
		filter: string;
		services: string[];
		label: string;
	};
	$: filters = invoke<LogFilter[]>('log_filters');

	const openLogInNewWindow = async (log: unknown) => {
		const key = await invoke<string>('kv_put', { value: JSON.stringify(log) });

		try {
			const existingWindow = WebviewWindow.getByLabel(key);

			if (existingWindow) {
				existingWindow.setFocus();
				return;
			}
			const view = new WebviewWindow(key, {
				url: `/window/log?kvKey=${key}`,
				minHeight: 900,
				minWidth: 1440,
				title: `${log['app'] ?? 'Unknown'} #${key}`
			});
			view.once('tauri://error', function (args) {
				console.warn('error', args);
			});
		} catch (e) {
			console.warn(e);
		}
	};

	let jsonViewNode: HTMLElement;
</script>

<svelte:head>
	<title>Logs</title>
	<meta name="description" content="Wombat" />
</svelte:head>

<div class="px-2">
	<div class="flex flex-col gap-2">
		<div class="flex h-full gap-4">
			Ping me for custom search filter templates.
			<div class="flex flex-wrap gap-2">
				<button
					class="btn btn-accent btn-xs"
					on:click={() => {
						timerange.set({
							type: 'relative',
							amount: 15,
							unit: 'minutes'
						});
					}}>Last 15m</button
				>
				<button
					class="btn btn-accent btn-xs"
					on:click={() => {
						timerange.set({
							type: 'relative',
							amount: 30,
							unit: 'hours'
						});
					}}>Last 30m</button
				>
				<button
					class="btn btn-accent btn-xs"
					on:click={() => {
						timerange.set({
							type: 'relative',
							amount: 1,
							unit: 'hours'
						});
					}}>Last 1h</button
				>
				<button
					class="btn btn-accent btn-xs"
					on:click={() => {
						timerange.set({
							type: 'relative',
							amount: 2,
							unit: 'hours'
						});
					}}>Last 2h</button
				>
				<button
					class="btn btn-accent btn-xs"
					on:click={() => {
						timerange.set({
							type: 'relative',
							amount: 4,
							unit: 'hours'
						});
					}}>Last 4h</button
				>
				<button
					class="btn btn-accent btn-xs"
					on:click={() => {
						timerange.set({
							type: 'relative',
							amount: 8,
							unit: 'hours'
						});
					}}>Last 8h</button
				>
				<button
					class="btn btn-accent btn-xs"
					on:click={() => {
						timerange.set({
							type: 'relative',
							amount: 24,
							unit: 'hours'
						});
					}}>Last 24h</button
				>
				<button
					class="btn btn-accent btn-xs"
					on:click={() => {
						timerange.set({
							type: 'absolute',
							from: startOfDay(new Date()),
							to: endOfDay(new Date())
						});
					}}>Today</button
				>
			</div>
			<div class="flex flex-wrap gap-2">
				{#await filters}
					<div>Loading filters...</div>
				{:then filters}
					{#each filters as filter}
						{#if filter.services.some((ls) => $selectedServices.some((ecs) => ecs.name == ls))}
							<button
								class="btn btn-active btn-secondary btn-xs"
								on:click={() => {
									filterString.set(filter.filter);
								}}>{filter.label}</button
							>
						{/if}
					{/each}
				{/await}
			</div>
		</div>
		<div class="flex gap-2">
			<div class="min-w-[200px]">
				<select class="w-full select-sm select select-bordered" bind:value={$activeCluser}>
					{#each $clusters as cluster}
						<option value={cluster}>{cluster.name}</option>
					{/each}
				</select>
			</div>
			<div class="grow">
				<ServiceMultiselect />
			</div>
			<div>
				<TimerangeSelect
					onSelect={(newRange) => {
						timerange.set(newRange);
					}}
					range={$timerange}
				/>
			</div>
		</div>
		<div class="w-full flex gap-2">
			<input
				type="text"
				placeholder="Filter"
				autocomplete="off"
				autocorrect="off"
				autocapitalize="off"
				spellcheck="false"
				class="input input-sm input-bordered grow"
				bind:value={$filterString}
			/>
			{#if !$storeState.isLookingForLogs}
				<button
					class="btn btn-sm btn-active btn-primary"
					disabled={$selectedServices.length === 0}
					on:click={() => {
						if ($selectedServices.length > 0 && $activeCluser?.env) {
							logStore.search(
								$selectedServices.map((s) => s.name),
								$activeCluser.env
							);
						}
					}}
					data-umami-event="logs_search_start"
					data-umami-event-uid={$userStore.id}
				>
					<svg
						xmlns="http://www.w3.org/2000/svg"
						fill="none"
						viewBox="0 0 24 24"
						stroke-width="1.5"
						stroke="currentColor"
						class="w-6 h-6"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z"
						/>
					</svg>

					Search!</button
				>
				<button
					class="btn btn-sm btn-active btn-primary"
					disabled={$selectedServices.length === 0}
					on:click={() => {
						if ($selectedServices.length > 0 && $activeCluser?.env) {
							logStore.dumpLogs(
								$selectedServices.map((s) => s.name),
								$activeCluser.env
							);
						}
					}}
					data-umami-event="logs_search_dump_start"
					data-umami-event-uid={$userStore.id}
				>
					<svg
						xmlns="http://www.w3.org/2000/svg"
						fill="none"
						viewBox="0 0 24 24"
						stroke-width="1.5"
						stroke="currentColor"
						class="w-6 h-6"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							d="M3 16.5v2.25A2.25 2.25 0 005.25 21h13.5A2.25 2.25 0 0021 18.75V16.5M16.5 12L12 16.5m0 0L7.5 12m4.5 4.5V3"
						/>
					</svg>

					Dump logs</button
				>
			{/if}
			{#if $storeState.isLookingForLogs}
				<button
					class="btn btn-sm btn-active btn-warning"
					on:click={() => {
						logStore.abort('user-request');
					}}
					data-umami-event="logs_search_stop"
					data-umami-event-uid={$userStore.id}
				>
					<svg
						xmlns="http://www.w3.org/2000/svg"
						fill="none"
						viewBox="0 0 24 24"
						stroke-width="1.5"
						stroke="currentColor"
						class="w-6 h-6"
					>
						<path
							stroke-linecap="round"
							stroke-linejoin="round"
							d="M15.75 5.25v13.5m-7.5-13.5v13.5"
						/>
					</svg>

					Abort</button
				>
			{/if}
		</div>
	</div>
</div>

<div class="p-1 min-h-6">
	{#if !!$storeState.searchError}{$storeState.searchError}{/if}
	{#if $storeState.isLookingForLogs}<progress
			class={`progress w-full ${$storeState.searchError ? 'progress-error' : ''}`}
		></progress>{/if}
	{#if !$storeState.isLookingForLogs}<progress
			class={`progress w-full
				${$storeState.searchStatus == 'aborted' ? 'progress-warning' : ''}
				${$storeState.searchStatus == 'success' ? 'progress-success' : ''}
				${$storeState.searchStatus == 'error' ? 'progress-error' : ''}`}
			value="100"
			max="100"
		></progress>{/if}
</div>
<div class="flex flex-col w-full gap-2">
	<div
		class={`overflow-auto ${
			$storeState.showLogDetails ? 'h-[calc(60vh-240px)]' : 'h-[calc(100vh-240px)]'
		} w-full`}
	>
		<table class="table table-xs w-full">
			<tbody>
				{#each $storeState.logs as log}
					<tr
						on:click={() => {
							logStore.showLog(log);
						}}
						class={`cursor-pointer text-white ${log.style.bg} ${log.style.hover} ${
							$selectedLog === log.data ? log.style.active : ''
						}`}
					>
						<td class="break-keep">{log.app}</td>
						<td>{log.level}</td>
						<td class="min-w-[170px] max-w-[170px]"
							>{format(new Date(log.timestamp), 'yyyy-MM-dd HH:mm:ss.SSS')}</td
						>
						<td>
							<table class="w-full table-fixed border-collapse border-0 border-spacing-0">
								<tbody>
									<tr class="border-0">
										<td class="text-ellipsis overflow-hidden whitespace-nowrap p-0 m-0">
											{log.message}
										</td>
									</tr>
								</tbody>
							</table>
						</td>
						<td class="">
							<div class="flex">
								<button
									class="p-1 -m-0.5 bg-base-100 rounded-full"
									data-umami-event="log_open_in_window"
									data-umami-event-uid={$userStore.id}
									on:click={(e) => {
										e.stopPropagation();
										openLogInNewWindow(log.data);
									}}
								>
									<svg
										xmlns="http://www.w3.org/2000/svg"
										viewBox="0 0 20 20"
										fill="currentColor"
										class="w-3 h-3"
									>
										<path
											fill-rule="evenodd"
											d="M4.25 5.5a.75.75 0 0 0-.75.75v8.5c0 .414.336.75.75.75h8.5a.75.75 0 0 0 .75-.75v-4a.75.75 0 0 1 1.5 0v4A2.25 2.25 0 0 1 12.75 17h-8.5A2.25 2.25 0 0 1 2 14.75v-8.5A2.25 2.25 0 0 1 4.25 4h5a.75.75 0 0 1 0 1.5h-5Z"
											clip-rule="evenodd"
										/>
										<path
											fill-rule="evenodd"
											d="M6.194 12.753a.75.75 0 0 0 1.06.053L16.5 4.44v2.81a.75.75 0 0 0 1.5 0v-4.5a.75.75 0 0 0-.75-.75h-4.5a.75.75 0 0 0 0 1.5h2.553l-9.056 8.194a.75.75 0 0 0-.053 1.06Z"
											clip-rule="evenodd"
										/>
									</svg>
								</button>
							</div></td
						>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
</div>
<div class="fixed w-full bottom-0 bg-transparent">
	<div class="w-full flex-col bg-base-300 rounded-t-lg">
		<div class="w-full flex flex-row justify-center relative pb-1">
			<div>
				{#if !$storeState.showLogDetails}
					<button on:click={() => ($storeState.showLogDetails = true)}>
						<svg
							xmlns="http://www.w3.org/2000/svg"
							fill="none"
							viewBox="0 0 24 24"
							stroke-width="1.5"
							stroke="currentColor"
							class="w-6 h-6"
						>
							<path stroke-linecap="round" stroke-linejoin="round" d="M4.5 15.75l7.5-7.5 7.5 7.5" />
						</svg>
					</button>
				{/if}
				{#if $storeState.showLogDetails}
					<button on:click={() => ($storeState.showLogDetails = false)}>
						<svg
							xmlns="http://www.w3.org/2000/svg"
							fill="none"
							viewBox="0 0 24 24"
							stroke-width="1.5"
							stroke="currentColor"
							class="w-6 h-6"
						>
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								d="M19.5 8.25l-7.5 7.5-7.5-7.5"
							/>
						</svg>
					</button>
				{/if}
			</div>
			{#if $storeState.showLogDetails}
				<div class="absolute right-2 top-1 flex gap-2 flex-row items-center">
					<button
						data-umami-event="log_open_in_window"
						data-umami-event-uid={$userStore.id}
						class="btn btn-circle btn-xs"
						on:click={() => openLogInNewWindow($selectedLog)}
					>
						<svg
							xmlns="http://www.w3.org/2000/svg"
							viewBox="0 0 20 20"
							fill="currentColor"
							class="size-5"
						>
							<path
								fill-rule="evenodd"
								d="M4.25 5.5a.75.75 0 0 0-.75.75v8.5c0 .414.336.75.75.75h8.5a.75.75 0 0 0 .75-.75v-4a.75.75 0 0 1 1.5 0v4A2.25 2.25 0 0 1 12.75 17h-8.5A2.25 2.25 0 0 1 2 14.75v-8.5A2.25 2.25 0 0 1 4.25 4h5a.75.75 0 0 1 0 1.5h-5Z"
								clip-rule="evenodd"
							/>
							<path
								fill-rule="evenodd"
								d="M6.194 12.753a.75.75 0 0 0 1.06.053L16.5 4.44v2.81a.75.75 0 0 0 1.5 0v-4.5a.75.75 0 0 0-.75-.75h-4.5a.75.75 0 0 0 0 1.5h2.553l-9.056 8.194a.75.75 0 0 0-.053 1.06Z"
								clip-rule="evenodd"
							/>
						</svg>
					</button>
				</div>
			{/if}
		</div>

		{#if $selectedLog && $storeState.showLogDetails}
			<div class={`h-[40vh] flex flex-col gap-2`}>
				<div class={`text-sm overflow-auto h-[40vh]`}>
					<div bind:this={jsonViewNode}>
						<JsonView log={$selectedLog} />
					</div>
				</div>
			</div>
		{/if}
		{#if !$selectedLog && $storeState.showLogDetails}
			<div class={`h-[40vh] overflow-auto justify-evenly text-center flex flex-col gap-2`}>
				Select log to see details
			</div>
		{/if}
	</div>
</div>
