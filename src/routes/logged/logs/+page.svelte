<script lang="ts">
	import { userStore } from '$lib/stores/user-store';
	import { JsonView } from '@zerodevx/svelte-json-view';
	import { endOfDay, format, startOfDay, sub } from 'date-fns';
	import { clusterStore } from '$lib/stores/cluster-store';
	import { serviceStore } from '$lib/stores/service-store';
	import { invoke } from '@tauri-apps/api';
	import { writeText } from '@tauri-apps/api/clipboard';
	import { beforeNavigate } from '$app/navigation';
	import { logStore } from '$lib/stores/log-store';
	import ServiceMultiselect from '$lib/componets/service-multiselect.svelte';

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
</script>

<svelte:head>
	<title>Logs</title>
	<meta name="description" content="Wombat" />
</svelte:head>

<div class="flex gap-8 content-end flex-col-reverse lg:flex-row px-2">

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
			{#if $selectedServices.some(s => s.name == 'rome')}
				<button
					class="btn btn-active btn-secondary btn-xs"
					on:click={() => {
						filterString.set(`{ $.level = "ERROR" }`);
					}}>Only Errors</button
				>
				<button
					class="btn btn-active btn-secondary btn-xs"
					on:click={() => {
						filterString.set(`{ $.mdc.traceId = "TRACE_ID_UUID" }`);
					}}>By Trace</button
				>
			{/if}
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
							logStore.search($selectedServices.map(s => s.name), $activeCluser.env);
						}
					}}
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
							logStore.dumpLogs($selectedServices.map(s => s.name), $activeCluser.env);
						}
					}}
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
			$storeState.showLogDetails ? 'h-[calc(60vh-260px)]' : 'h-[calc(100vh-260px)]'
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
<div class="fixed w-full bottom-0">
	<div class="w-full flex-col bg-base-300 rounded-t-lg">
		<div class="w-full flex justify-center">
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
						<path stroke-linecap="round" stroke-linejoin="round" d="M19.5 8.25l-7.5 7.5-7.5-7.5" />
					</svg>
				</button>
			{/if}
		</div>

		{#if $selectedLog && $storeState.showLogDetails}
			<div class="h-[40vh] flex flex-col gap-2">
				<button
					class="m-2 btn btn-active btn-primary btn-sm"
					on:click={async () => {
						await writeText(JSON.stringify($selectedLog, null, 2));
					}}>Copy raw json</button
				>
				<div class="text-sm overflow-auto h-[calc(40vh-80px)]">
					<JsonView json={$selectedLog} />
				</div>
			</div>
		{/if}
		{#if !$selectedLog && $storeState.showLogDetails}
			<div class="overflow-auto h-[40vh] justify-evenly text-center flex flex-col gap-2">
				Select log to see details
			</div>
		{/if}
	</div>
</div>
