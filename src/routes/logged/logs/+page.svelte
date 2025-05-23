<script lang="ts">
	import { add, endOfDay, format, startOfDay, sub } from 'date-fns';
	import { clusterStore } from '$lib/stores/cluster-store';
	import { serviceStore } from '$lib/stores/service-store';
	import { invoke } from '@tauri-apps/api/core';
	import { beforeNavigate } from '$app/navigation';
	import { logStore, type LogData } from '$lib/stores/log-store';
	import ServiceMultiselect from '$lib/componets/service-multiselect.svelte';
	import { userStore } from '$lib/stores/user-store';
	import JsonView from '$lib/componets/json-view.svelte';
	import { WebviewWindow } from '@tauri-apps/api/webviewWindow';
	import TimerangeSelect from '$lib/componets/timerange-select.svelte';
	import { logFiltersStore } from '$lib/stores/log-filters-store';

	let activeCluser = clusterStore.activeCluser;
	let selectedServices = serviceStore.selectedServices;
	let clusters = clusterStore.clusters;

	let timerange = logStore.timerange;
	let filterString = logStore.filterString;
	let selectedLog = logStore.selectedLog;
	let storeState = logStore.storeState;

	beforeNavigate(async () => {
		invoke('abort_find_logs', { reason: 'navigation' });
	});

	const openLogInNewWindow = async (log: LogData) => {
		const key = await invoke<string>('kv_put', { value: JSON.stringify(log) });

		try {
			const existingWindow = await WebviewWindow.getByLabel(key);

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
			view.once('tauri://error', function (args: unknown) {
				console.warn('error', args);
			});
		} catch (e) {
			console.warn(e);
		}
	};

	let jsonViewNode: HTMLElement | undefined = $state();
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
					onclick={() => {
						timerange.set({
							type: 'relative',
							amount: 15,
							unit: 'minutes'
						});
					}}>Last 15m</button
				>
				<button
					class="btn btn-accent btn-xs"
					onclick={() => {
						timerange.set({
							type: 'relative',
							amount: 30,
							unit: 'minutes'
						});
					}}>Last 30m</button
				>
				<button
					class="btn btn-accent btn-xs"
					onclick={() => {
						timerange.set({
							type: 'relative',
							amount: 1,
							unit: 'hours'
						});
					}}>Last 1h</button
				>
				<button
					class="btn btn-accent btn-xs"
					onclick={() => {
						timerange.set({
							type: 'relative',
							amount: 2,
							unit: 'hours'
						});
					}}>Last 2h</button
				>
				<button
					class="btn btn-accent btn-xs"
					onclick={() => {
						timerange.set({
							type: 'relative',
							amount: 4,
							unit: 'hours'
						});
					}}>Last 4h</button
				>
				<button
					class="btn btn-accent btn-xs"
					onclick={() => {
						timerange.set({
							type: 'relative',
							amount: 8,
							unit: 'hours'
						});
					}}>Last 8h</button
				>
				<button
					class="btn btn-accent btn-xs"
					onclick={() => {
						timerange.set({
							type: 'relative',
							amount: 1,
							unit: 'days'
						});
					}}>Last 1d</button
				>
				<button
					class="btn btn-accent btn-xs"
					onclick={() => {
						timerange.set({
							type: 'relative',
							amount: 3,
							unit: 'days'
						});
					}}>Last 3d</button
				>
				<button
					class="btn btn-accent btn-xs"
					onclick={() => {
						timerange.set({
							type: 'relative',
							amount: 7,
							unit: 'days'
						});
					}}>Last 7d</button
				>
				<button
					class="btn btn-accent btn-xs"
					onclick={() => {
						timerange.set({
							type: 'absolute',
							from: startOfDay(new Date()),
							to: endOfDay(new Date())
						});
					}}>Today</button
				>
			</div>
			<div class="flex flex-wrap gap-2">
				{#if $logFiltersStore.isLoading}
					<div>Loading filters...</div>
				{:else}
					{@const filters = $logFiltersStore.filters}
					{#each filters as filter (filter.id)}
						{@const enabledFor = filter.services.filter((ls) => ls.at(0) !== '!')}
						{@const disabledFor =
							filter.services
								.find((ls) => ls.at(0) === '!')
								?.substring(1)
								?.split(',') ?? []}
						{@const matches =
							enabledFor.some((ls) => $selectedServices.some((ecs) => ecs.name === ls)) ||
							$selectedServices.some(
								(ecs) => disabledFor.length > 0 && !disabledFor.includes(ecs.name)
							)}
						{#if matches}
							<button
								class="btn btn-active btn-secondary btn-xs"
								onclick={() => {
									filterString.set(filter.filter);
								}}>{filter.label}</button
							>
						{/if}
					{/each}
				{/if}
			</div>
		</div>
		<div class="flex gap-2">
			<div class="min-w-[200px]">
				<select class="w-full select-sm select" bind:value={$activeCluser}>
					{#each $clusters as cluster (cluster.arn)}
						<option value={cluster}>{cluster.name}</option>
					{/each}
				</select>
			</div>
			<div class="grow">
				<ServiceMultiselect />
			</div>
			<div>
				<TimerangeSelect range={timerange} />
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
					onclick={() => {
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
					onclick={() => {
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
					onclick={() => {
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

<div class="relative flex items-center min-h-6">
	{#if $storeState.isLookingForLogs}<progress
			class={`progress w-full ${$storeState.searchStatus == 'error' ? 'progress-error' : ''}`}
		></progress>{/if}
	{#if !$storeState.isLookingForLogs}<progress
			class={`progress w-full
				${$storeState.searchStatus == 'aborted' ? 'progress-warning' : ''}
				${$storeState.searchStatus == 'success' ? 'progress-success' : ''}
				${$storeState.searchStatus == 'error' ? 'progress-error' : ''}`}
			value="100"
			max="100"
		></progress>{/if}
	{#if !!$storeState.message}
		<div class="absolute top-0 left-0 w-full flex justify-center">
			<span
				class={`px-4 rounded-md bg-base-300 text-sm border
				${$storeState.searchStatus === undefined ? 'border-divider' : ''}
     			${$storeState.searchStatus == 'aborted' ? 'border-warning' : ''}
     			${$storeState.searchStatus == 'success' ? 'border-success' : ''}
     			${$storeState.searchStatus == 'error' ? 'border-error' : ''}
			`}
			>
				{$storeState.message}
			</span>
		</div>
	{/if}
</div>
<div class="flex flex-col w-full gap-2">
	<div
		class={`overflow-auto ${
			$storeState.showLogDetails ? 'h-[calc(60vh-240px)]' : 'h-[calc(100vh-240px)]'
		} w-full`}
	>
		<table class="table table-xs w-full">
			<tbody>
				{#each $storeState.logs as log (log.id)}
					<tr
						onclick={() => {
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
									onclick={(e) => {
										e.preventDefault();
										openLogInNewWindow(log.data);
									}}
									aria-label="Open log info"
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
					<button onclick={() => ($storeState.showLogDetails = true)} aria-label="Open log details">
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
					<button
						onclick={() => ($storeState.showLogDetails = false)}
						aria-label="Hide log details"
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
						onclick={() => {
							if ($selectedLog) {
								openLogInNewWindow($selectedLog);
							}
						}}
						aria-label="Open log window"
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
			<div class="h-[40vh] flex flex-col gap-2">
				<div class="text-sm overflow-auto h-[40vh]">
					<div bind:this={jsonViewNode}>
						<JsonView
							log={selectedLog}
							createFilter={(prop, value) => {
								if (prop === 'timestamp') {
									logStore.timerange.set({
										type: 'absolute',
										from: sub(new Date((value + 'Z') as string), { minutes: 1 }),
										to: add(new Date((value + 'Z') as string), { minutes: 1 })
									});
									return;
								}
								logStore.filterString.update((s) => {
									if (s.length > 1 && s[0] == '[') {
										return s;
									}
									const oldFilters =
										s.trim() == '' ? '' : s.substring(1, s.length - 2).trim() + ' &&';
									return `{ ${oldFilters} $.${prop} = "${value}" }`;
								});
							}}
						></JsonView>
					</div>
				</div>
			</div>
		{/if}
		{#if !$selectedLog && $storeState.showLogDetails}
			<div class="h-[40vh] overflow-auto justify-evenly text-center flex flex-col gap-2">
				Select log to see details
			</div>
		{/if}
	</div>
</div>
