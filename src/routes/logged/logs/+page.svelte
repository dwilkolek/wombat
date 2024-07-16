<script lang="ts">
	import { endOfDay, format, startOfDay, sub } from 'date-fns';
	import { clusterStore } from '$lib/stores/cluster-store';
	import { serviceStore } from '$lib/stores/service-store';
	import { invoke } from '@tauri-apps/api';
	import { writeText } from '@tauri-apps/api/clipboard';
	import { beforeNavigate } from '$app/navigation';
	import { logStore } from '$lib/stores/log-store';
	import ServiceMultiselect from '$lib/componets/service-multiselect.svelte';
	import { userStore } from '$lib/stores/user-store';
	import JsonView from '$lib/componets/json-view.svelte';

	$: activeCluser = clusterStore.activeCluser;

	$: selectedServices = serviceStore.selectedServices;

	$: clusters = clusterStore.clusters;
	const toLocalDateStr = (date: Date) => {
		return `${date.getUTCFullYear()}-${withZeros(date.getMonth() + 1)}-${withZeros(
			date.getDate()
		)}T${withZeros(date.getHours())}:${withZeros(date.getMinutes())}:${withZeros(
			date.getSeconds()
		)}`;
	};
	const withZeros = (v: number) => {
		return v < 10 ? `0${v}` : v;
	};

	$: startDate = logStore.startDate;
	$: endDate = logStore.endDate;
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

	$: expanded = false;

	$: logHeight = expanded ? `h-[calc(24vh)]` : `h-[calc(60vh-240px)]`;
	$: logHeightContainer = expanded ? 'h-[74vh]' : 'h-[40vh]';
	$: logJsonViewHeight = expanded ? 'h-[74vh]' : 'h-[40vh]';
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
						startDate.set(sub(new Date(), { minutes: 5 }));
						endDate.set(new Date());
					}}>Last 5m</button
				>
				<button
					class="btn btn-accent btn-xs"
					on:click={() => {
						startDate.set(sub(new Date(), { minutes: 15 }));
						endDate.set(new Date());
					}}>Last 15m</button
				>
				<button
					class="btn btn-accent btn-xs"
					on:click={() => {
						startDate.set(sub(new Date(), { minutes: 30 }));
						endDate.set(new Date());
					}}>Last 30m</button
				>
				<button
					class="btn btn-accent btn-xs"
					on:click={() => {
						startDate.set(sub(new Date(), { hours: 1 }));
						endDate.set(new Date());
					}}>Last 1h</button
				>
				<button
					class="btn btn-accent btn-xs"
					on:click={() => {
						startDate.set(sub(new Date(), { hours: 4 }));
						endDate.set(new Date());
					}}>Last 4h</button
				>
				<button
					class="btn btn-accent btn-xs"
					on:click={() => {
						startDate.set(sub(new Date(), { hours: 8 }));
						endDate.set(new Date());
					}}>Last 8h</button
				>
				<button
					class="btn btn-accent btn-xs"
					on:click={() => {
						startDate.set(sub(new Date(), { hours: 24 }));
						endDate.set(new Date());
					}}>Last 24h</button
				>
				<button
					class="btn btn-accent btn-xs"
					on:click={() => {
						startDate.set(startOfDay(new Date()));
						endDate.set(endOfDay(new Date()));
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
				<input
					type="datetime-local"
					placeholder="Start date"
					class="input input-sm input-bordered w-full max-w-xs"
					on:change={(event) => {
						startDate.set(new Date(event.currentTarget.value));
					}}
					value={toLocalDateStr($startDate)}
				/>
			</div>
			<div>
				<input
					type="datetime-local"
					placeholder="End date"
					class="input input-sm input-bordered w-full max-w-xs"
					on:change={(event) => {
						endDate.set(new Date(event.currentTarget.value));
					}}
					value={toLocalDateStr($endDate)}
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
			$storeState.showLogDetails ? logHeight : 'h-[calc(100vh-240px)]'
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
						<td>{log.app}</td>
						<td>{log.level}</td>
						<td class="min-w-[200px] max-w-[200px]"
							>{format(new Date(log.timestamp), 'yyyy-MM-dd HH:mm:ss.SSS')}</td
						>
						<td class="w-full">{log.message}</td>
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
				<div class="absolute right-2 -top-1 flex gap-2 flex-row items-center">
					<button
						class="m-2 btn btn-active btn-primary btn-xs"
						on:click={async () => {
							await writeText(JSON.stringify($selectedLog, null, 2));
						}}>Copy raw json</button
					>
					<button
						class="btn btn-circle btn-xs"
						on:click={() => {
							expanded = !expanded;
						}}
					>
						{#if expanded}
							<svg
								xmlns="http://www.w3.org/2000/svg"
								viewBox="0 0 20 20"
								fill="currentColor"
								class="size-5"
							>
								<path
									d="M3.28 2.22a.75.75 0 0 0-1.06 1.06L5.44 6.5H2.75a.75.75 0 0 0 0 1.5h4.5A.75.75 0 0 0 8 7.25v-4.5a.75.75 0 0 0-1.5 0v2.69L3.28 2.22ZM13.5 2.75a.75.75 0 0 0-1.5 0v4.5c0 .414.336.75.75.75h4.5a.75.75 0 0 0 0-1.5h-2.69l3.22-3.22a.75.75 0 0 0-1.06-1.06L13.5 5.44V2.75ZM3.28 17.78l3.22-3.22v2.69a.75.75 0 0 0 1.5 0v-4.5a.75.75 0 0 0-.75-.75h-4.5a.75.75 0 0 0 0 1.5h2.69l-3.22 3.22a.75.75 0 1 0 1.06 1.06ZM13.5 14.56l3.22 3.22a.75.75 0 1 0 1.06-1.06l-3.22-3.22h2.69a.75.75 0 0 0 0-1.5h-4.5a.75.75 0 0 0-.75.75v4.5a.75.75 0 0 0 1.5 0v-2.69Z"
								/>
							</svg>
						{:else}
							<svg
								xmlns="http://www.w3.org/2000/svg"
								viewBox="0 0 20 20"
								fill="currentColor"
								class="size-5"
							>
								<path
									d="m13.28 7.78 3.22-3.22v2.69a.75.75 0 0 0 1.5 0v-4.5a.75.75 0 0 0-.75-.75h-4.5a.75.75 0 0 0 0 1.5h2.69l-3.22 3.22a.75.75 0 0 0 1.06 1.06ZM2 17.25v-4.5a.75.75 0 0 1 1.5 0v2.69l3.22-3.22a.75.75 0 0 1 1.06 1.06L4.56 16.5h2.69a.75.75 0 0 1 0 1.5h-4.5a.747.747 0 0 1-.75-.75ZM12.22 13.28l3.22 3.22h-2.69a.75.75 0 0 0 0 1.5h4.5a.747.747 0 0 0 .75-.75v-4.5a.75.75 0 0 0-1.5 0v2.69l-3.22-3.22a.75.75 0 1 0-1.06 1.06ZM3.5 4.56l3.22 3.22a.75.75 0 0 0 1.06-1.06L4.56 3.5h2.69a.75.75 0 0 0 0-1.5h-4.5a.75.75 0 0 0-.75.75v4.5a.75.75 0 0 0 1.5 0V4.56Z"
								/>
							</svg>
						{/if}
					</button>
				</div>
			{/if}
		</div>

		{#if $selectedLog && $storeState.showLogDetails}
			<div class={`${logHeightContainer} flex flex-col gap-2`}>
				<div class={`text-sm overflow-auto ${logJsonViewHeight}`}>
					<JsonView log={$selectedLog} />
				</div>
			</div>
		{/if}
		{#if !$selectedLog && $storeState.showLogDetails}
			<div
				class={`${logHeightContainer} overflow-auto justify-evenly text-center flex flex-col gap-2`}
			>
				Select log to see details
			</div>
		{/if}
	</div>
</div>
